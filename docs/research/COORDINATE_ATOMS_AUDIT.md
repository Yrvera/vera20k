# Coordinate System Atoms — gamemd.exe vs Rust Engine Audit

Every foundational piece ("atom") of the positioning and coordinate system,
compared between gamemd.exe (verified via Ghidra) and our Rust engine.

Status: MATCH = verified identical, APPROX = close but not exact,
MISMATCH = known difference, UNVERIFIED = not yet checked in gamemd.

---

## 1. World coordinate system (leptons)

The 3D coordinate system used internally for all positions.

| Property | gamemd.exe | Rust engine | Status |
|----------|-----------|-------------|--------|
| Leptons per cell | 256 | 256 | MATCH |
| Cell center sub-cell | (128, 128) | CELL_CENTER_LEPTON = 128 | MATCH |
| X axis direction | Southeast | Southeast | MATCH |
| Y axis direction | Southwest | Southwest | MATCH |
| Z axis direction | Up | Up | MATCH |
| LevelHeight | 104 leptons (verified: cot(60°) × 256√2 × 0.5) | Not used — we use height levels directly | APPROX |
| CellHeight | 208 leptons (2 × LevelHeight) | Not used | APPROX |

**Issue**: We represent Z as integer height levels (0-14), not leptons.
gamemd stores Z in leptons internally. When gamemd needs Z for projection,
it uses the actual lepton value, which allows sub-level precision (slopes,
ramps). We multiply height level × 15 for screen offset, losing the
fractional precision that leptons provide.

---

## 2. CoordsToClient — isometric projection (THE core function)

Converts 3D lepton position to 2D screen pixels.

| Property | gamemd.exe (0x6D2140) | Rust engine | Status |
|----------|----------------------|-------------|--------|
| X formula | `(X*60/2 + Y*(-60)/2) >> 8` | `(rx-ry) * 30.0` (for cell coords) | MATCH |
| Y formula (before Z) | `(X*30/2 + Y*30/2) >> 8` | `(rx+ry) * 15.0 + 15.0` (includes cell center) | MATCH |
| Z subtraction | `- AdjustForZ(Z_leptons)` | `- z * 15.0` (height level × 15) | APPROX |
| Integer rounding | Truncation toward zero (C integer `/`) | Float arithmetic (f32) | APPROX |
| Sub-cell precision | Full lepton precision in projection | `lepton_sub_to_screen_offset` for sub-cell | MATCH |

**Issue**: The Z subtraction uses different methods. See atom #3.

---

## 3. AdjustForZ — height to screen Y conversion

Converts a Z value in leptons to a screen pixel Y offset.

| Property | gamemd.exe (0x6D20E0) | Rust engine | Status |
|----------|----------------------|-------------|--------|
| Formula | `ftol(Z × 0.14348 + (Z≥728?1:0) + 0.5)` | `z * 15.0` (height level × constant) | APPROX |
| Input | Z in leptons (continuous) | Z as height level (integer 0-14) | MISMATCH |
| Per height level | 104 × 0.14348 = 14.922 → rounds to 15 | 15.0 exactly | APPROX |
| Cumulative error at z=4 | ftol(416 × 0.14348 + 0.5) = ftol(60.19) = 60 | 4 × 15 = 60 | MATCH |
| Cumulative error at z=8 | ftol(832 × 0.14348 + 1 + 0.5) = ftol(120.87) = 120 | 8 × 15 = 120 | MATCH |
| Sub-level Z (slopes) | Handles fractional heights | No sub-level support | MISMATCH |
| Multiplier constant | DAT_00b0cd48 ≈ 0.14348 | Not used (hardcoded 15) | APPROX |
| Threshold correction | +1 pixel when Z ≥ 728 leptons (≈7 levels) | None | MISMATCH |

**Issue**: For exact height levels (z=0,1,2,...), our z×15 matches gamemd's
AdjustForZ to within 1px. But for:
- Slopes/ramps with fractional height: we have no sub-level Z
- Heights ≥ 7: gamemd adds +1 pixel correction, we don't
- Art.ini Height values: gamemd does `Height × HeightFactor(104)` in leptons,
  then AdjustForZ; we do `Height × 15` directly

**Fix needed**: Implement AdjustForZ properly with the lepton-based formula
and the ≥728 correction. Use `height_level × 104` to convert to leptons first.

---

## 4. iso_to_screen — tile positioning

Where tile images are drawn on screen.

| Property | gamemd.exe (FUN_006d7560 → FUN_00480350) | Rust engine | Status |
|----------|------------------------------------------|-------------|--------|
| X formula | `30*(rx-ry) - 30` | `(rx-ry)*30 - 30` | MATCH |
| Y formula | `15*(rx+ry) + 15 - z*15` | `(rx+ry)*15 + 15 - z*15` | MATCH |
| Tile anchor | NW corner of 60×30 bounding box | NW corner of 60×30 bounding box | MATCH |

**Status**: MATCH (fixed this session).

---

## 5. lepton_to_screen — entity positioning

Where entities (buildings, units) are positioned on screen.

| Property | gamemd.exe (CoordsToClient) | Rust engine | Status |
|----------|---------------------------|-------------|--------|
| X formula | `30*(rx-ry)` | `(rx-ry)*30` | MATCH |
| Y formula | `15*(rx+ry) + 15 - AdjustForZ(z)` | `(rx+ry)*15 + 15 - z*15` | APPROX |
| Sub-cell offset | Included in lepton input to CoordsToClient | `lepton_sub_to_screen_offset` added separately | MATCH |

**Issue**: Same AdjustForZ approximation as atom #3.

---

## 6. lepton_sub_to_screen_offset — sub-cell movement

Converts sub-cell lepton offsets to screen pixel offsets for smooth movement.

| Property | gamemd.exe | Rust engine | Status |
|----------|-----------|-------------|--------|
| X formula | `(dx - dy) * 30 / 256` (via CoordsToClient) | `(dx - dy) * (60/256) / 2` | MATCH |
| Y formula | `(dx + dy) * 15 / 256` (via CoordsToClient) | `(dx + dy) * (30/256) / 2` | MATCH |
| Integer rounding | Truncation in CoordsToClient | Float (f32) | APPROX |

**Status**: Functionally correct. Minor float vs int rounding differences.

---

## 7. SHP canvas centering

How sprite images are positioned relative to their anchor point.

| Property | gamemd.exe (DrawSHP 0x4AED70, flag 0x200) | Rust engine | Status |
|----------|-------------------------------------------|-------------|--------|
| X centering | `draw_x -= canvas_w / 2` (integer) | `offset_x = -(full_w / 2) as f32` | MATCH |
| Y centering | `draw_y -= canvas_h / 2` (integer) | `offset_y = -(full_h / 2) as f32` | MATCH |
| Frame offset | `draw_x += frame_rect.x; draw_y += frame_rect.y` | Baked into composited canvas at (fx, fy) | MATCH |
| Canvas source | SHP header bytes 2-5 | `shp.width`, `shp.height` from same header | MATCH |
| Division type | C integer division (truncate toward zero) | Rust u32 division (same truncation) | MATCH |

**Status**: MATCH. Verified with actual runtime data (GACNST canvas=284×226).

---

## 8. XDrawOffset / YDrawOffset handling

Art.ini per-type draw offset adjustments.

| Property | gamemd.exe | Rust engine | Status |
|----------|-----------|-------------|--------|
| XDrawOffset | Added as direct pixel X offset | Added as direct pixel X offset | MATCH |
| YDrawOffset (buildings) | Added to CellClass.ZLevel, then through AdjustForZ (Z-scaled) | Added as direct pixel Y offset | MISMATCH |
| YDrawOffset (units) | Added as direct pixel Y offset to draw position | Added as direct pixel Y offset | MATCH |
| Default value | 0 for most buildings | 0 for most buildings | MATCH |

**Issue**: For buildings, gamemd treats YDrawOffset as a Z-height adjustment
(in the same units as terrain height), not as direct pixels. It goes through
the `AdjustForZ` conversion. A YDrawOffset of 10 in gamemd shifts by
`AdjustForZ(10) ≈ 1.4 pixels`, but in our engine it shifts by 10 pixels.

Most buildings have YDrawOffset=0 so this doesn't matter, but some special
buildings or effects with non-zero values would be rendered at wrong heights.

**Fix needed**: For buildings, convert YDrawOffset through the Z projection
instead of adding as direct pixels.

---

## 9. HeightFactor — art.ini Height to Z leptons

Converts the art.ini `Height=` value to Z leptons for building dimensions.

| Property | gamemd.exe | Rust engine | Status |
|----------|-----------|-------------|--------|
| HeightFactor value | 104 (= LevelHeight, stored at 0x89DDB8) | Not used explicitly | N/A |
| Height to Z | `Height × 104` leptons | `Height × 15` screen pixels | APPROX |
| Used for | Dimension2().Z, health bar Z projection | Health bar pip Y offset | APPROX |

**Issue**: Our health bar uses `height * 15` (PIP_HEIGHT_FACTOR).
gamemd uses `AdjustForZ(Height × 104)`. For integer heights these are
very close (both ≈15 per unit), but not identical due to the 14.92 vs 15
factor and the ≥728 threshold correction.

---

## 10. Terrain height representation

How terrain elevation is stored and used.

| Property | gamemd.exe | Rust engine | Status |
|----------|-----------|-------------|--------|
| Storage | CellClass+0x11B: signed byte (height level) | `z: u8` height level | MATCH |
| Range | Typically 0-14 | 0-14 | MATCH |
| To screen pixels | Through AdjustForZ(level × LevelHeight) | `z * 15.0` (HEIGHT_STEP) | APPROX |
| Slope/ramp Z | Sub-level precision via lepton Z coords | Integer height levels only | MISMATCH |
| Bridge height | +4 levels (field 0x140 bit 7) | Separate bridge height map | MATCH |

**Issue**: Slopes and ramps in gamemd can have fractional height values
(in leptons), giving smooth Z transitions. Our integer height levels mean
units/buildings on slopes snap to discrete heights, potentially causing
1-7px visual jumps at slope boundaries.

---

## 11. TMP tile sub-tile offsets

Multi-cell TMP templates (cliffs, ramps) have per-sub-tile draw offsets.

| Property | gamemd.exe | Rust engine | Status |
|----------|-----------|-------------|--------|
| extra_x byte offset | Sub-tile header +20 (piVar4[5]) | `raw_extra_x` at +20 | MATCH |
| extra_y byte offset | Sub-tile header +24 (piVar4[6]) | `raw_extra_y` at +24 | MATCH |
| extra_width/height | +28, +32 (piVar4[7], piVar4[8]) | `extra_width/height` at +28, +32 | MATCH |
| Flags byte | +36 (piVar4[9] low byte) | `flags` at +36 | MATCH |
| Template→tile conversion | `piVar4[5] - piVar4[0]` at render time | `raw_extra_x - (col-row)*tw/2` at parse time | MATCH |
| Signed interpretation | C signed int | Rust i32 | MATCH |
| Units | Pixels (direct) | Pixels (direct) | MATCH |
| Rendering approach | Two separate blits (diamond + extra) | Pre-composite into enlarged buffer + offset | MATCH (same pixel output) |

**Status**: MATCH. Verified via Ghidra decompilation of FUN_00547cf0 (tile blitter)
and FUN_00547020 (TMP loader). Byte offsets, signedness, units, and semantic
application all match.

---

## 12. Depth sorting / draw order

How objects are sorted front-to-back for correct occlusion.

| Property | gamemd.exe | Rust engine | Status |
|----------|-----------|-------------|--------|
| Tile depth | `height_level * -15 - 2` | `compute_entity_depth(...)` | UNVERIFIED |
| Building depth | Foundation bottom Y based | Foundation bottom Y based | APPROX |
| Unit depth | Screen Y of feet | Screen Y + sprite bottom | APPROX |
| Per-pixel Z (terrain) | TMP Z-data per tile | zdepth shader (terrain/overlays only) | MATCH |
| Per-pixel Z (buildings) | BUILDNGZ loaded but ignored by blitter | Removed (single depth per sprite) | MATCH |

---

## Summary of issues to fix (priority order)

### ~~P1: AdjustForZ (atom #3)~~ — NOT AN ISSUE
For integer height levels, `z * 15` produces IDENTICAL results to
`AdjustForZ(z * 104)`. The 14.922 factor rounds to exactly 15 for every
integer multiple. The ≥728 threshold correction also lands cleanly at
level 7 (728 leptons). No fix needed unless sub-level Z precision is added.

### ~~P2: YDrawOffset Z-scaling for buildings (atom #8)~~ — NOT AN ISSUE
Only 5 entries in art.ini/artmd.ini use YDrawOffset, all are
animations/explosions (FIRSTRM1, water explosions, BIGBLUE). Zero buildings
have non-zero YDrawOffset. No fix needed.

### P3: Slope/ramp sub-level Z (atoms #3, #10)
Impact: Visual snapping on slopes instead of smooth height transitions.
Fix: Store Z in leptons (or fixed-point) instead of integer height levels.
This is a larger architectural change.

### P4: TMP sub-tile offset verification (atom #11)
Impact: Cliff and ramp tiles could be misaligned.
Fix: Compare our TMP parser output with gamemd's for a known cliff tile.

### Low priority
- Integer rounding differences (atoms #2, #6): sub-pixel, not visible
- Depth sorting details (atom #12): visual-only, not positioning
