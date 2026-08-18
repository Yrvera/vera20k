# Coordinate System — gamemd.exe vs Rust Engine

## Overview

This document records the exact coordinate system used by gamemd.exe for isometric
rendering, traced via Ghidra MCP decompilation. The goal is to make our Rust engine
replicate gamemd.exe's coordinate pipeline exactly — no convention offsets, no
compensations, just the same algorithms.

**Patch note 2026-05-22:** verify-doc-swarm corrected several stale
claims in this document. Terrain tile rendering projects cell origin,
not cell center; `BuildingClass::GetCoords` uses foundation width for X
and foundation height for Y; the top `CoordsToClient` pseudocode must
include viewport subtraction/bounds behavior; and the older "BEFORE
fix" Rust guidance below is historical, not a current patch plan.

## gamemd.exe Coordinate Pipeline (verified)

### CoordsToClient — the core projection

Address: `0x6D2140` (TacticalClass::CoordsToClient2)

Converts 3D lepton coordinates to 2D client pixels. The isometric
projection is computed first, then tactical viewport scroll is
subtracted and a visibility/bounds flag is returned:

```c
worldX = (((X * 60) / 2) + ((Y * -60) / 2)) >> 8;  // truncation toward zero
worldY = ((((X * 30) / 2) + ((Y * 30) / 2)) >> 8) - AdjustForZ(Z);
screenX = worldX - Tactical->ScrollX;  // Tactical+0xB0
screenY = worldY - Tactical->ScrollY;  // Tactical+0xB4
return in_bounds_with_tactical_margins(screenX, screenY);
```

For a cell at (rx, ry) with cell origin leptons (rx\*256, ry\*256):
- `screenX = 30*(rx - ry)`
- `screenY = 15*(rx + ry)`

For cell center leptons (rx\*256+128, ry\*256+128):
- `screenX = 30*(rx - ry)`  ← same X! The +128 cancels in (X-Y)
- `screenY = 15*(rx + ry) + 15`  ← +15 from (128+128)*15/256

### Tile rendering

Traced via Ghidra: FUN_006d7560 → CoordsToClient2(cell_center) → FUN_00480350.
2026-05-22 correction: `0x006D7560` and `0x006D6D10` build
`cell*256+0x80`, then truncate back to `cell*256` before
`CoordsToClient`. gamemd projects **cell origin** for tile positioning,
not cell center. The safe formula is:

```
tile_draw_x = CoordsToClient(cell_origin).X - 30 = 30*(rx-ry) - 30
tile_draw_y = CoordsToClient(cell_origin).Y      = 15*(rx+ry) - z*15
```

The prior center-based wording here was wrong; use the origin formula above.
The tile image NW corner is drawn at:

```
tile_draw_x = CoordsToClient(cell_origin).X - 30 = 30*(rx-ry) - 30
tile_draw_y = CoordsToClient(cell_origin).Y       = 15*(rx+ry) - z*15
```

The -30 X shifts from the diamond north vertex (CoordsToClient.X) to the bounding
box NW corner. The tile path does not include the +15 cell-center Y offset.

This equals: `(30*(rx-ry) - 30, 15*(rx+ry) - z_screen)`

Verified: cell content rendering at `FUN_006D6D10` applies `-0x1E` (-30) to
CoordsToClient.X before drawing.

### Building sprite rendering

Traced through: TacticalClass → BuildingClass::DrawBody (0x43D290) →
TechnoClass::DrawSHP (0x705E00) → DrawSHP (0x4AED70)

**No +30 X or -15 Y offsets anywhere in the chain.** The building sprite is drawn at:

```
draw_pos = CoordsToClient(building_anchor) - viewport
```

With flag 0x200, DrawSHP applies canvas centering:
```
draw_x -= canvas_width / 2    (integer division)
draw_y -= canvas_height / 2   (integer division)
draw_x += frame_rect.x        (SHP frame offset within canvas)
draw_y += frame_rect.y
```

2026-05-22 correction: do not treat `vtable+0xAC` / pLocation as the
general building body draw anchor. Audited building draw and target
paths use `+0x1B8` and `+0x48` contexts; `+0xAC` appears in adjacent
anim/object anchor contexts. The older simplified wording below is
historical context and should not drive implementation.

Historical wording claimed that the building anchor comes from
`vtable+0xAC` (GetAnchorCoords), which for buildings returns the
pLocation (cell center leptons). That is not the general building body
draw contract; use the correction above. The old simplified derivation was:

```
building_screen_x = 30*(rx - ry)
building_screen_y = 15*(rx + ry) + 15 - z_screen
```

Relative to tile:
```
entity_x - tile_x = 30*(rx-ry) - (30*(rx-ry) - 30) = +30
entity_y - tile_y = (15*(rx+ry) + 15) - 15*(rx+ry) = +15
```

The entity is at (+30, +15) from the tile NW corner = the tile diamond center.
This is correct — building sprites are anchored at the cell center.

### Health bar pip rendering

Address: `0x6F64A0` (TechnoClass::DrawHealthBar)

For buildings (WhatAmI == 6), receives pLocation (cell center screen coords):

```
pip0.X = pLoc.X + screen1.X + 3 + numPips*4
pip0.Y = pLoc.Y + screen1.Y + 4 - numPips*2
step per pip: (-4, +2)
```

Where:
- `screen1 = CoordsToClient(-foundW*128, +foundH*128, Height*HeightFactor)`
- `numPips = (screen1.Y - screen2.Y) / 2` ← integer division
- `HeightFactor = 104` (= LevelHeight, verified via initialization chain at 0x45B080)

Simplified (for even foundH):
```
pip0.X = pLoc.X + 15*(fh - fw) + 3
pip0.Y = pLoc.Y + 4 - 7.5*(fw + fh) - AdjustForZ(Height * 104)
```

AdjustForZ(Height * 104) ≈ Height * 15 (since 104 * b0cd48 ≈ 14.92, rounds to 15).

### AdjustForZ

Address: `0x6D20E0`

```c
z_screen = ftol(Z_leptons * DAT_00b0cd48 + (Z >= 728 ? 1 : 0) + 0.5)
```

Where `DAT_00b0cd48 ≈ 0.14348` (= sin(60°) * 60 / (sqrt(2) * 256)).

Per height unit: `104 * 0.14348 ≈ 14.92 ≈ 15 pixels`.

### HeightFactor

Address: stored at `0x89DDB8`, initialized at `0x45B070`
(corrected 2026-05-28: was `0x45B080`; binary function entry is `0x45B070` via `get_function_by_address 0x45B070` — ROOT_CAUSE: GHIDRA_ADDRESS_SHIFT)

```
HeightFactor = ftol(cot(60°) * 256*sqrt(2) * 0.5)
             = ftol(0.5774 * 362.04 * 0.5)
             = ftol(104.52)
             = 104 = LevelHeight
```

## Historical Rust Guidance (stale)

2026-05-22 status: this section is stale implementation guidance. Do
not bulk-apply the file list or compensation-removal plan without
rechecking current Rust. Treat it as historical context for why the
coordinate convention was investigated; use current source scans and
the corrected tile-origin/building-center facts above for new work.

### iso_to_screen (tile position)
```rust
sx = (rx - ry) * 30       // gamemd: (rx - ry) * 30 - 30
sy = (rx + ry) * 15       // gamemd: same
```
**Tiles are 30px too far right.**

### lepton_to_screen (entity position)
```rust
sx = (rx - ry) * 30 + 30  // gamemd: (rx - ry) * 30 — the +30 compensates for tile shift
sy = (rx + ry) * 15 + 15  // gamemd: same
```
**+30 is a compensation, not present in gamemd.**

### Building sprite Y
```rust
final_y = sy - TILE_HEIGHT / 2.0 + offset_y  // -15 compensates for tile Y convention
```
**-15 is a compensation, not present in gamemd.**

### Health bar X
```rust
start_x = sx + 15*(fh - fw) - 27  // -27 = gamemd's +3 minus the +30 compensation
```
**-27 is a compensation, not present in gamemd (should be +3).**

## Historical Fix Proposal (stale)

2026-05-22 status: historical proposal only. The audited correction is
that tile rendering projects cell origin and subtracts `0x1E` from X,
while building/object anchors require separate `+0x1B8`, `+0x48`, and
target-coordinate handling. Re-scan current Rust before using any
affected-file list below.

Change `iso_to_screen` to subtract 30 from X, matching the `-0x1E` in gamemd's
cell rendering. Then all compensations become unnecessary:

### After fix

| Component | Formula | Matches gamemd? |
|-----------|---------|-----------------|
| `iso_to_screen` X | `(rx-ry)*30 - 30` | Yes |
| `lepton_to_screen` X | `(rx-ry)*30` | Yes (= CoordsToClient) |
| Building sprite Y | `sy + offset_y` (no -15) | Yes |
| Health bar X | `sx + 15*(fh-fw) + 3` | Yes |
| Overlay X | `screen_x + offset_x` (no +30) | Yes |
| Ghost placement X | `iso_x + offset_x` (no +30) | Yes |

### Files affected by iso_to_screen change

**Core:**
- `src/map/terrain.rs` — `iso_to_screen()` definition
- `src/util/lepton.rs` — `lepton_to_screen()` remove +30

**Building sprites (remove -TILE_HEIGHT/2):**
- `src/app_instances/shp.rs` — building body, depth, turret, bib, anims
- `src/app_instances/overlays.rs` — fog snapshots

**Overlays (remove +TILE_WIDTH/2 from X):**
- `src/app_instances/overlays.rs` — FX, overlay sprites, terrain objects
- `src/app_render.rs` — smudge rendering
- `src/render/selection_overlay.rs` — ghost placement

**Health bar:**
- `src/app_ui_overlays.rs` — change -27 to +3

**Click detection / camera:**
- `src/map/terrain.rs` — `screen_to_iso()` inverse function
- `src/app_input.rs` — mouse click to cell conversion
- `src/render/minimap.rs` — minimap coordinate mapping
- `src/render/minimap_helpers.rs` — minimap normalization

**Tests:**
- `src/map/terrain.rs` — iso_to_screen test assertions
- `src/util/lepton.rs` — lepton test assertions
- `src/sim/game_entity.rs` — screen position assertions
- `src/sim/movement_tests.rs` — movement position assertions
- `src/sim/world_tests.rs` — world spawn assertions

## Key constants reference

| Constant | gamemd value | Meaning |
|----------|-------------|---------|
| Cell width (pixels) | 60 (0x3C) | Full tile diamond width |
| Cell height (pixels) | 30 (0x1E) | Full tile diamond height |
| Tile NW offset X | -30 (-0x1E) | North vertex to NW corner |
| Cell center Y offset | +15 | Sub-cell (128,128) projection |
| HeightFactor | 104 | = LevelHeight = cot(60°) * 256√2 * 0.5 |
| AdjustForZ multiplier | ~0.14348 | = sin(60°) * pixelPerLepton |
| AdjustForZ per Height | ~14.92 ≈ 15 | pixels per art.ini Height unit |
| AdjustForZ threshold | 728 (0x2D8) | adds +1 pixel when Z >= 728 |
| Pip step | (-4, +2) | per pip along NW edge |
| Pip X constant | +3 | from DrawHealthBar |
| Pip Y constant | +4 | from DrawHealthBar |

## Building placement foundation shadow

### Rendering pipeline

Parent function: `FUN_006d5030` (called from TacticalClass::Draw)
Per-cell function: `FUN_0047ec90`
Special extension functions: `FUN_006d5730` (LaserFencePost), `FUN_006d59d0` (FirestormWall), `FUN_006d5c50` (Overlay walls)

### Per-cell coordinate math

```c
// In FUN_006d5030 — for each foundation cell (cellX, cellY):

// 1. Convert cell to leptons + 128, then truncate back to cell origin
lepton_x = cellX * 256 + 128;
lepton_y = cellY * 256 + 128;
coord_x = ((lepton_x + sign_corr) >> 8) << 8;  // = cellX * 256 (truncates +128)
coord_y = ((lepton_y + sign_corr) >> 8) << 8;  // = cellY * 256
coord_z = 0;

// 2. CoordsToClient → tile north point
screen = CoordsToClient(coord_x, coord_y, 0);
//   screen.x = (cellX - cellY) * 30
//   screen.y = (cellX + cellY) * 15

// 3. Subtract viewport scroll, subtract 30 from X
pos_x = screen.x - scroll_x - 30;   // -30 passed to per-cell function
pos_y = screen.y - scroll_y;
```

```c
// In FUN_0047ec90 — per-cell rendering:

height = cell->Height;  // CellClass offset 0x11B (signed byte)

draw_x = pos_x + 30;              // +30 restores: net 0 from CoordsToClient
draw_y = pos_y + height * -15 - 1;  // height adjustment + 1px nudge

// Frame selection:
//   bit 0x04 set (normal placement) → frame 1 (valid/green)
//   cell valid from FUN_0047ec90 check → frame 0 (connectable)
//   neither → frame (2 + slope_flag) (invalid/red)

DrawSHP(PLACE_SHP, frame, {draw_x, draw_y}, surface,
        flags=0x20600, 0,
        z_depth = height * -15 - 2 - (slope ? 10 : 0),
        0, 1000, 0, 0, 0, 0, 0);
```

### DrawSHP flag 0x200 canvas centering (from 0x4AED70)

```c
if (flags & 0x200) {
    x -= canvas_width / 2;   // integer division
    y -= canvas_height / 2;  // integer division
}
x += frame_x;  // SHP frame offset within canvas
y += frame_y;
```

### PLACE.SHP frame layout

Canvas: 60×59 pixels. Frame 0 (valid): frame_x=0, frame_y=30, frame_w=60, frame_h=29.

With flag 0x200 canvas centering applied:
```
final_x = draw_x - 60/2 + 0 = (tile_north_x + 30 - 30 + 30) - 30 + 0 = tile_north_x
final_y = draw_y - 59/2 + 30 = (tile_north_y - h*15 - 1) - 29 + 30 = tile_north_y - h*15
```
The -1 Y nudge is absorbed by integer truncation: 59/2 = 29 (not 29.5), and frame_y=30.

### Comparison with Rust engine

| Aspect | gamemd.exe | Rust engine | Match? |
|--------|-----------|-------------|--------|
| Base position | CoordsToClient(cell_origin) | iso_to_screen(crx, cry, z) | Yes |
| X final | tile_north_x | tile_north_x | Yes |
| Y final | tile_north_y - height*15 | tile_north_y - z*15 | Yes |
| Diamond size | 60×29 via SHP frame | 60×30 quad (1px stretch) | ~1px |
| Frame selection | 3 frames (valid/connect/invalid) | 2-way split + tinting | Different approach |
| Z-depth | height*-15 -2 (-10 if slope) | flat DRAG_RECT_DEPTH | Different |

### Foundation direction table (DAT_0089f688)

Runtime-initialized at `0x0049f2f0`. 8 entries of 4 bytes (two int16: dx, dy):

| Index | dx | dy | Direction |
|-------|----|----|-----------|
| 0 | 0 | -1 | North |
| 1 | 1 | -1 | NorthEast |
| 2 | 1 | 0 | East |
| 3 | 1 | 1 | SouthEast |
| 4 | 0 | 1 | South |
| 5 | -1 | 1 | SouthWest |
| 6 | -1 | 0 | West |
| 7 | -1 | -1 | NorthWest |

Used by LaserFencePost/FirestormWall/Wall extension shadow rendering only.
Regular foundation cells use a bounds-based iteration from FUN_004a94f0.

## Building GetCoords / GetTargetCoords

### Virtual function table (BuildingClass vtable at 0x7E3EBC)

| Offset | Address | Function |
|--------|---------|----------|
| +0x28 | 0x44E8F0 | GetType |
| +0x48 | 0x447AC0 | **GetCoords** (returns foundation center) |
| +0xA4 | 0x4500A0 | **GetTargetCoords** (GetCoords + TargetCoordOffset) |
| +0x1B8 | 0x41BEA0 | GetCell (Location / 256, for rendering) |

### GetCoords (vtable+0x48) — FUN_00447ac0

Returns foundation center for buildings, raw Location for other objects:
```c
result.X = Location.X + (foundation_width - 1) * 128;
result.Y = Location.Y + (foundation_height - 1) * 128;
result.Z = Location.Z;
```
Where Location = NW corner cell center (cell * 256 + 128).

### GetTargetCoords (vtable+0xA4) — FUN_004500a0

Adds TargetCoordOffset from BuildingTypeClass:
```c
coords = GetCoords();  // foundation center
result.X = coords.X + BuildingType->TargetCoordOffset.X;  // +0xEBC
result.Y = coords.Y + BuildingType->TargetCoordOffset.Y;  // +0xEC0
result.Z = coords.Z + BuildingType->TargetCoordOffset.Z;  // +0xEC4
```

Only 3 buildings have non-zero TargetCoordOffset (all naval yards):
- GAYARD (Allied): 300, 200, 0
- NAYARD (Soviet): 256, 256, 0
- YAYARD (Yuri): 300, 200, 0

### GetCell (vtable+0x1B8) — FUN_0041bea0

Used by render path. Returns raw Location converted to cell integer:
```c
result = pack( (Location.X + sign_corr) >> 8,
               (Location.Y + sign_corr) >> 8 );
```
Truncates +128 sub-cell offset — returns NW corner cell number.

## AnimClass::DrawIt — ZAdjust usage

Address: `0x422CA0` (458 lines)

ZAdjust is at AnimClass offset 0x100 (= `param_1[0x40]`).
YDrawOffset is at AnimTypeClass offset 0x344 (= `AnimType->YDrawOffset`).

### Z-depth parameter to DrawSHP

Pattern appears 3 times (corrected 2026-07-18: prior text said "4 times" without
listing a 4th variant; re-decompiled `0x422CA0` this session and found exactly 3
occurrences of `param_1[0x40]` (ZAdjust) combined with the
`YDrawOffset - AdjustForZ` shape — matching the three variants listed below, not
four. A 4th, differently-shaped z-depth calc exists in the same function (gated by
`param_1[0x32]+0x372`, computed as `-2 - AdjustForZ()` with no YDrawOffset/ZAdjust
term), but it is not an instance of this pattern. Verified via
`decompile_function 0x422CA0` — ROOT_CAUSE: INFERENCE_HARDENED):
```c
z_depth = (AnimType->YDrawOffset + this->ZAdjust) - AdjustForZ(...) - constant;
```

Three variants:
```c
// Standard animation (0x369 flag not set):
z_depth = (YDrawOffset + ZAdjust - AdjustForZ(height)) - 2;

// Alternate rendering (0x369 flag set):
z_depth = (YDrawOffset + ZAdjust - AdjustForZ(height)) - 3;

// Tiled/looping animation (0x35B flag set):
z_depth = (YDrawOffset + ZAdjust - AdjustForZ(height)) - 50;  // 0x32
```

### Screen Y position

```c
// YDrawOffset is added to screen Y for sprite positioning:
screen_y = param_2[1] + AnimType->YDrawOffset;
```

### Our engine gap

(corrected 2026-07-18: this was a blanket claim; the current split status per
consumer, verified by reading current `src/` this session, is:)

- **Still a gap** — general building body animations (art.ini `ActiveAnim`/
  `IdleAnim`, e.g. power plant smoke, war factory lights): `emit_building_anims`
  (`src/app_instances/shp.rs:548-696`) reads `anim.x`/`anim.y` for position but
  never reads `anim.z_adjust`; depth is always the flat `building_depth`
  (`src/app_instances/shp.rs:683`, comment: "YSortAdjust... affects draw ORDER
  in the original, not depth"). Verified by reading `emit_building_anims` this
  session — no `z_adjust` reference in the function body.
- **Still a gap** — building turret VXL (`TurretAnimZAdjust`):
  `emit_building_turret_vxl` (`src/app_instances/shp.rs:347-395`) takes the value
  as parameter `_z_adjust: i32` (underscore-prefixed = deliberately unused) and
  never applies it to the computed screen position. Verified by reading the full
  function body this session.
- **No longer a gap for this one consumer** — garrison weapon-muzzle-flash
  animations: `garrison_flash_depth_apply_z_adjust`
  (`src/app_instances/overlays.rs:564-568`) applies `z_adjust` as a depth-sort
  bias, called live from the per-flash draw loop
  (`src/app_instances/overlays.rs:533-539`, not test-only). This is a narrower
  fix than the general-anim gap described above — RUST_IMPL_SUPERSEDED for this
  specific path only.

This still affects Z-buffer depth / draw order of building body animations and
building turret VXLs; only the garrison-flash path has been closed.

## CellToPixel vs CoordsToClient

### TacticalClass::CellToPixel (0x6D1FE0)

```c
result.x = ((X * 60) / 2 + (Y * -60) / 2 + sign_corr) >> 8;
result.y = ((X * 30) / 2 + (Y * 30) / 2 + sign_corr) >> 8;
```
Same isometric formula as CoordsToClient but:
- **No Z/height adjustment**
- **No viewport scroll subtraction**
- **No bounds checking**

Pure lepton-to-pixel projection. CoordsToClient adds height, scroll, and bounds on top.

### TacticalClass::CoordsToClient2 (0x6D2140)

```c
raw_x = ((X * 60) / 2 + (Y * -60) / 2 + sign_corr) >> 8;
raw_y = ((X * 30) / 2 + (Y * 30) / 2 + sign_corr) >> 8;
height_px = AdjustForZ(Z);
result.x = raw_x - viewport_scroll_x;
result.y = raw_y - height_px - viewport_scroll_y;
// Returns visibility flag (in bounds check with 360px/180px margin)
```

### AdjustForZ (0x6D20E0) — assembly-level detail

```asm
SUB ESP, 8
CMP ECX, 0x2D8           ; 728 leptons threshold
MOV [ESP+4], ECX          ; store Z input
MOV [ESP], 0              ; extra = 0
JL skip
MOV [ESP], 1              ; extra = 1 if Z >= 728
skip:
FILD [ESP+4]              ; push Z as float
FMUL [0xB0CD48]           ; * scale_factor (BSS, ~0.14348)
FIADD [ESP]               ; + extra
FADD [0x7E1738]           ; + 0.5 (rounding constant, confirmed = 0x3FE0...)
CALL ftol                 ; truncate to int
```

Result: `ftol(Z * scale + (Z >= 728 ? 1 : 0) + 0.5)`

Our engine uses discrete `z * 15`. Matches for standard heights but could differ
by ±1px at unusual Z values (ramps, bridges with intermediate heights).

## Ghidra addresses reference

| Address | Function |
|---------|----------|
| 0x6D1FE0 | TacticalClass::CellToPixel (no Z, no scroll) |
| 0x6D2140 | TacticalClass::CoordsToClient2 (full projection) |
| 0x6D20E0 | AdjustForZ (height to screen Y) |
| 0x6D5030 | Building placement overlay renderer (parent) |
| 0x47EC90 | Building placement per-cell draw (PLACE.SHP) |
| 0x6D5730 | LaserFencePost placement extension shadow |
| 0x6D59D0 | FirestormWall placement extension shadow |
| 0x6D5C50 | Overlay wall placement extension shadow |
| 0x6D6D10 | Cell content rendering (has -0x1E X offset) |
| 0x6D8DB0 | Object rendering loop (tactical) |
| 0x43D290 | BuildingClass::DrawBody |
| 0x422CA0 | AnimClass::DrawIt (ZAdjust usage) |
| 0x447AC0 | BuildingClass::GetCoords (foundation center) |
| 0x4500A0 | BuildingClass::GetTargetCoords |
| 0x41BEA0 | BuildingClass::GetCell (NW corner for rendering) |
| 0x705E00 | TechnoClass::DrawSHP |
| 0x4AED70 | Low-level DrawSHP (canvas centering with flag 0x200) |
| 0x6F64A0 | TechnoClass::DrawHealthBar |
| 0x45B070 | HeightFactor initialization (corrected 2026-05-28: was 0x45B080; verified via get_function_by_address — GHIDRA_ADDRESS_SHIFT) |
| 0x7E3EBC | BuildingClass vtable base |
| 0x89DDB8 | HeightFactor storage |
| 0x89F688 | Foundation direction table (8 directions, runtime init) |
| 0x8A03FC | PLACE.SHP global pointer |
| 0xB0CD48 | AdjustForZ multiplier storage |
