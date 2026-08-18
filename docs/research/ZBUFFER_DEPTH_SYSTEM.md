# gamemd.exe Z-Buffer Depth System

Reverse-engineered via Ghidra MCP (live decompilation of Yuri's Revenge `gamemd.exe`).
Documents the per-pixel Z-buffer system used for depth ordering between sprites.

---

## Overview

The original engine uses a **16-bit per-pixel Z-buffer** (`DAT_00887644`) that determines
visibility at every pixel. Only **terrain tiles** actively read+write the Z-buffer per pixel:

```c
// TMP tiles (per-pixel from tile Z-data) — the ONLY active Z-buffer read+write path:
pixel_z = z_shape_value + base_z;
if (pixel_z <= zbuffer[pixel]) {         // <= for terrain
    zbuffer[pixel] = pixel_z;
    screen[pixel] = color;
}
```

**Critical finding:** SHP sprite blitters selected through `TechnoClass::DrawSHP`
(which always ORs `0x800`) do **NOT** read or write the Z-buffer per pixel. The
per-scanline blitter functions at vtable offsets 0x74/0x78/0x130 (selected when
`0x800` is set) perform alpha compositing only — Z-buffer pointer and Z-value
parameters are passed but **completely ignored** by these leaf functions.

Z-writing blitters exist (e.g. `0x00497100` at offset 0x10c, `0x00495bc0` at
vtable `0x007e5600`) but are **unreachable** through the normal flag dispatch when
`0x800` is set. They appear to be dead code for standard game object rendering.

**Depth ordering for SHP sprites relies on:**
1. **Terrain Z-values** — written by `TMP_TileBlitter` in Phase 1 (terrain pass)
2. **Screen-Y sort order** — objects within Phase 2 are sorted by screen position
3. **Layer ordering** — 5 display layers rendered in sequence

Lower Z = closer to camera. Terrain tiles use `<=` (can overwrite at equal depth).

**Z-mode flags** (from `vtable+0x68` / `GetVisualState` dispatch in `TechnoClass::DrawSHP`):

| Return | Z-flags | Visual state | Blitter selected (with 0x800) |
|--------|---------|-------------|-------------------------------|
| **0** | `0x00` | **Normal (opaque)** | Offset 0x124: `Blitter_Opaque_RLE_Remap` (`0x004978c0`) — opaque, no Z |
| 1 | `0x02` | Uncloaking (early) | Offset 0x12c: translucent, Z-read |
| 2 | `0x04` | Uncloaking (late) | Offset 0x130: 50% blend, no Z-write |
| 3 | `0x04` | Cloaking (visible) | Same as case 2 |
| 4 | varies | Cloaking (depends) | `param_1[0x89]` flag → 0x02 or 0x04 |
| 5 | — | Fully cloaked | Draw skipped |

**ALL normal (non-cloaked) objects return 0** — buildings, infantry, vehicles, and
aircraft. The dispatch chain: `BuildingClass_GetVisualState` (`0x004544a0`,
delegates when `+0x6ED = 0`) or `FootClass::GetVisualState` (`0x004da4e0`,
asks locomotor first) → `TechnoClass_GetVisualState` (`0x00703860`) → returns 0
when CloakState (`this[0x88]`) is 0. The only non-zero returns come from:
- **Cloaking/uncloaking** (CloakState != 0) → returns 1–5 based on progress
- **TunnelLocomotionClass** in burrowed state → returns 4 or 5

The blitter at offset 0x124 (vtable `0x007e5470`, function `Blitter_Opaque_RLE_Remap`
at `0x004978c0`) performs **opaque RLE rendering** — direct pixel assignment with
remap/intensity lookup, no Z-buffer read or write, no alpha blend.

Cases 2/3 select **50% translucent blitters** (for cloaking visual effects only).

**Confirmed across all ~100 CC_Draw_Shape call sites:** every caller that sets
Z-bits (0x02/0x04/0x06) also sets 0x800. The Z-writing blitters at offset 0x10c
are **dead code** — unreachable in normal gameplay.

The Z-buffer is cleared to `0xFFFF` per dirty rect (not the whole surface)
via `FUN_007bcfb0`, called from `FUN_006d2b60` which iterates the dirty rect
list at `DAT_00b0ce7c`. Three separate subsystems provide the Z-shape depth data:

1. **TMP Z-data** — per-pixel depth baked into terrain tile files
2. **BUILDNGZ.SHA** — per-pixel depth overlay for building sprites
3. **Per-scanline Z-gradient table** — row-by-row depth accumulator

---

## 1. Z-Buffer Infrastructure

### Z-Buffer Surface

Allocated as a 0x30-byte object via `operator_new`, constructed by `FUN_007bc970`,
stored in global `DAT_00887644`. Created during game init in `FUN_006bb9a0`.

| Field | Address/Value | Purpose |
|-------|--------------|---------|
| Global pointer | `DAT_00887644` | 16-bit per-pixel depth surface |
| Origin X | `+0x00` | Screen/map origin X |
| Y origin | `+0x04` | Top of viewport in Z-buffer coords |
| Width | `+0x08` | Buffer width |
| Height | `+0x0C` | Buffer height |
| State/flags | `+0x10` | Initialized to 0 |
| Surface ptr | `+0x14` | Inner BSurface object (0x20 bytes) |
| Buffer start | `+0x18` | Pointer to pixel data start |
| Buffer end | `+0x1C` | Pointer to pixel data end (wrap check) |
| Buffer wrap | `+0x20` | Wrap-around size for circular buffer |
| Default Z | `+0x24` = `0x8000` (-32768 as signed short) | Base Z value for depth computation |
| Stride | `+0x28` | Width in pixels (uint16 elements per row) |
| Stride height | `+0x2C` | Buffer height (copy) |

### A-Buffer (Alpha/Auxiliary)

| Field | Address/Value | Purpose |
|-------|--------------|---------|
| Global pointer | `DAT_0087e8a4` | 16-bit auxiliary surface (intensity/remap overlay) |

### Base Z Formula

The base Z depends on the rendering path. Three variants exist:

**Tile renderer** (`FUN_00547cf0` — terrain/walls/ore):
```c
base_z = (ushort)(DefaultZ + YOrigin - screenY - spriteHeight)
         - (spriteHeight * heightLevel) / 2;
```

**SHP blitter, gradient field[5]=1** (entries 0, 1 — standard objects):
```c
raw = (ushort)(DefaultZ + YOrigin - screenY_offset) + z_height;
base_z = (raw / field[1]) * field[1];   // quantized (entry 0: field[1]=1, no-op)
```

**SHP blitter, gradient field[5]=0** (entry 2 — buildings):
```c
step = field[3] / field[2];             // entry 2: 3/1 = 3
raw = (ushort)(DefaultZ + YOrigin - spriteHeight - screenY_offset + 1) + z_height;
base_z = (raw / step) * step - spriteHeight / step;
accum_init = field[3] - spriteHeight % step;
if (accum_init == field[3]) { accum_init = 0; base_z += step_dir; }
```

- `DefaultZ` = `0x8000` (-32768 as signed short; set in constructor `FUN_007bc970`)
- `YOrigin` = Z-buffer `+0x04` (viewport top)
- `screenY` / `screenY_offset` = sprite's screen position components
- `spriteHeight` = draw rect height in pixels
- `z_height` = elevation-based Z parameter from caller
- Building formula subtracts spriteHeight and quantizes to 3-unit boundaries
- **Tile `heightLevel`**: `CellClass+0x11B` (signed char, typically 0–14). Moves tile
  upward by `heightLevel * 15` pixels. The actual Z formula parameter is
  `cellZAdjust` (`CellClass+0x10C`, signed short) — runtime-computed from
  `heightLevel * gradient_factor - offset`, scaled by tile intensity factor
  (16.16 fixed-point at `CellClass+0x104`, default `0x10000` = 1.0).
  Initialization: `FUN_00484680` during map load.
- **Animation `z_height`**: `YDrawOffset + ZAdjust - AdjustForZ() - 2` (non-flat)
  or `- 3` (flat). `ZAdjust` from art.ini stored at `AnimTypeClass+0x348`,
  instance copy at `AnimClass+0x100`.

Objects further DOWN the screen get LOWER Z values (closer to camera).

### Z-Buffer Flags in Draw Calls

The draw flags parameter controls Z-buffer behavior via **bits 1–2**:

| Bit | Value | Meaning |
|-----|-------|---------|
| 1 | `0x02` | Z-buffer read/test enabled |
| 2 | `0x04` | Z-buffer write enabled |
| 1+2 | `0x06` | Both read and write (cloaked/warping objects) |
| 4 | `0x10` | Z-shape overlay present (set in CC_Draw_Shape when `param_7 != 0`) |
| 11 | `0x800` | Always set by `TechnoClass::DrawSHP`; selects intensity-aware blitters |

The blitter selector (`FUN_00490b90`) dispatches on `flags & 6` to pick the
correct blitter object (read-only, write-only, or read+write Z modes),
then further selects based on `0x800`, `0x3000` (bits 12–13), and `0x4000`.

Note: `TechnoClass::DrawSHP` also ORs `0x200` (bit 9) and `0x800` (bit 11)
into the flags before calling `CC_Draw_Shape`. Bit 9 (`0x200`) controls
**sprite centering** (subtracts half SHP frame width from X and half frame
height from Y, verified at `0x004af002`: `TEST AH, 0x2`). Bit 10 (`0x400`)
is not tested in CC_Draw_Shape or the blitter selector.

---

## 2. TMP Z-Data (Terrain Tiles and Overlays)

### Source

Z-data is stored directly in the **TMP file format**, inside each sub-tile's cell header.

### TMP Cell Header (relevant fields)

| Offset | Size | Field |
|--------|------|-------|
| 12 | u32 | Relative byte offset from cell header to Z-data block |
| 36 | u8 | Flags — bit 1 (`0x02`) = has Z-data |

### Renderer: `FUN_00547cf0` at `0x00547cf0`

This is the per-cell tile renderer for 60×29 isometric diamond sprites (terrain tiles,
wall overlays, ore overlays). It renders TMP/overlay pixel data within the isometric
diamond defined by three mask tables:

| Address | Table | Purpose |
|---------|-------|---------|
| `DAT_00abb120` | Pixel data offsets | Per-scanline offset into tile frame data |
| `DAT_00aa074c` | Screen buffer offsets | Per-scanline offset into screen/zbuf scanline |
| `DAT_00aa154c` | Scanline widths | Number of pixels per scanline row |

### Z-Data Access

```c
piVar4 = frame_pointer;
if ((*(byte *)(piVar4 + 9) & 2) != 0) {       // flag bit 1 at byte 36
    z_shape_ptr = (byte *)(piVar4[3] + (int)piVar4);  // Z-data at offset 12
}
```

### Per-Pixel Z Test (inner loop at ~line 275)

```c
uint16_t pixel_z = (uint16_t)*z_shape_ptr + (int16_t)base_z;
if (pixel_z <= *zbuffer_ptr) {
    *zbuffer_ptr = pixel_z;                    // update Z-buffer
    *screen_ptr = remap[abuf_pixel | pixel];   // draw remapped pixel
}
z_shape_ptr++;
zbuffer_ptr++;
screen_ptr++;
```

### Status in Rust Engine

Our TMP parser (`src/assets/tmp_decode.rs`) reads Z-data with `FLAG_HAS_Z_DATA`
(`0x02`). The Z-data is used in the GPU depth pipeline via `zdepth_shader.wgsl`
which samples an R8 depth atlas for per-pixel terrain depth (cliff occlusion).

---

## 3. BUILDNGZ.SHA (Building Z-Shape Overlay)

### Source

A **separate SHP file** called `BUILDNGZ.SHA` stored in the MIX archives provides
per-pixel depth offsets for building sprites. Runtime-verified dimensions:
**396×477 pixels**, 169,488 non-zero pixels. This is a single large depth gradient
shared across all buildings (centered and bottom-aligned per building sprite).

### Loading: `FUN_0045e8f0` at `0x0045e8f0`

1. Loaded from MIX via `LoadFileFromMIX()` (`0x005b40b0`) into global `DAT_0089ddbc`
2. Gets frame 0 data via `FUN_0069e740(0)`, then for every non-zero pixel
   in `width × height` bytes (SHP header dimensions), the value is adjusted:
   ```c
   pixel -= 0x41;  // subtract 65 (byte arithmetic)
   ```
3. This converts raw bytes centered around 65 into signed depth offsets.
   The result is stored as a byte but read as `char` (signed) by the Z-test
   blitter, so the effective range is `[-64..+127]` for input bytes `[1..192]`.
   Input bytes above 192 wrap to negative values due to signed char overflow.

### Usage in Building Rendering

Buildings are rendered via:
```
BuildingClass_DrawBody (0x0043d290)
  → TechnoClass_DrawSHP (0x00705e00, RET 0x40 = 17 params)
    → CC_Draw_Shape (0x004aed70, RET 0x38 = 16 params)
      → SHP_StandardBlitter (0x004373b0) or SHP_ExtendedBlitter (0x00437a10)
```

The blitter choice is per-frame: `SHP_GetFrameCompressionFlag()` (`0x0069e900`) checks bit 1 of the SHP frame's
flag byte (offset 8 in the 24-byte frame header). Bit 1 set → extended blitter;
bit 1 clear → standard blitter (typical for building SHPs). Both blitters use
the gradient table, so gradient entry 2 applies in either path.

In `CC_Draw_Shape` (`FUN_004aed70`), when the Z-shape SHP pointer (`param_13`,
actually the BUILDNGZ.SHA pointer itself) is non-null:

```c
// Ghidra shows param_12 but real param is CC param_13 (BUILDNGZ.SHA pointer)
// due to 16-param function with Ghidra showing only 15.
// Assembly: TEST EBX, EBX at 0x004aef1c where EBX = [entry+0x2C] = CC param_13
if (buildngz_ptr != 0) {
    // ECX = BUILDNGZ.SHA SHP pointer (for SHP_Resolve inside)
    // param_14 = frame index (0 for buildings)
    piVar3 = (int *)SHP_GetFrameRect(&iStack_6c, frame_index);
    puStack_94 = operator_new(0x20);   // Z-shape draw context
    PixelBuffer_Init(uVar4, iStack_74 * iStack_70);  // prepare frame data
}
```

**IMPORTANT: Unreachable in normal building rendering.** The Z-shape data is loaded
and the context is allocated, but the per-scanline blitter selected for buildings
(offset 0x130, vtable `0x007e5440`, function `0x00497cf0`) **ignores the Z-shape
parameter entirely**. It performs 50% alpha blending without any Z-buffer access.

The Z-writing blitter at offset 0x10c (`Blitter_ZClip_Plain16_WritesZ`, `0x00497100`)
would use the BUILDNGZ data for per-pixel Z-tests:
```c
if (base_z - *z_shape_ptr < *zbuffer_ptr) {
    *dest = remap_table[pixel_index * 2];   // write color
    *zbuffer_ptr = base_z - *z_shape_ptr;   // update zbuffer
}
z_shape_ptr++;
```
However, offset 0x10c is **never returned** by `Blitter_selector_extended` when
`0x800` is set (which `TechnoClass::DrawSHP` always sets). This Z-writing path
is effectively dead code for normal game rendering.

### Building Draw Call Parameters

From `BuildingClass::DrawBody`:

```c
FUN_00705e00(
    shape,          // param_2: building SHP
    frame,          // param_3: frame index
    ...,
    z_height,       // param_8: base Z from screen position
    2,              // param_9: gradient type (entry 2 = buildings)
    1,              // param_10: consumed in TechnoClass, NOT forwarded to CC_Draw_Shape
    ...,
    DAT_0089ddbc,   // param_13: BUILDNGZ.SHA pointer (→ CC param_13, null check = Z-shape flag)
    0,              // param_14: Z-shape frame index (→ CC param_14 → FUN_0069e7e0)
    ...
);
```

Verified via full assembly trace: the 14-push sequence at `0x007065a3` maps
TechnoClass param_9 (=2) → CC_Draw_Shape **param_11** (gradient type), and
TechnoClass param_8 (z_height) → CC_Draw_Shape **param_10** (z_height).
TechnoClass param_13 (=DAT_0089ddbc) → CC_Draw_Shape param_13 (Z-shape SHP
pointer, tested as `EBX != 0`). TechnoClass param_14 (=0) → CC param_14
(frame index for `FUN_0069e7e0`). The gradient/z_height mapping was verified
from `FUN_004373b0` assembly: `ESP+0x78` (CC param_11) indexes the gradient
table, while `ESP+0x74` (CC param_10) is added to the Z formula as z_height.

### SpecialZOverlay (Gates)

Individual building types can have their own Z-overlay SHP files via the art.ini
key `SpecialZOverlay` (stored at type offset `+0x1510`). A companion key
`SpecialZOverlayZAdjust` (string at `0x0081a67c`) adjusts the Z-position.
Only gate buildings use this:

- `GAGATEZA` / `GAGATEZB` (Allied gates)
- `NAGATEZA` / `NAGATEZB` (Soviet gates)

### Status in Rust Engine

`BUILDNGZ.SHA` **is loaded and used** via `load_buildngz()` in
`src/render/sprite_atlas.rs`. The depth data is blitted into an R8 depth atlas
by `blit_buildngz_depth()` and sampled per-pixel by `zdepth_shader.wgsl`.

**Differences from original engine:**
- **Frame index: MATCHES.** Both engines use frame 0. The original engine passes
  frame index 0 as CC_Draw_Shape param_14 (verified from assembly at `0x007065f4`:
  `PUSH EDX` where EDX = `[TechnoClass entry+0x34]` = param_14 = 0). The loader
  (`FUN_0045e8f0`) also remaps frame 0 via `FUN_0069e740(0)`.
- **Missing -65 remap.** The original engine subtracts 0x41 from each non-zero
  BUILDNGZ pixel in `FUN_0045e8f0`, converting raw bytes to signed depth offsets.
  Our loader uses raw SHP pixel values directly as unsigned depth in the shader
  (`base_depth - z_sample * 0.0002`), while the original uses signed char
  subtraction (`base_z - (signed char)z_shape`). This produces different depth
  distributions — our engine treats all non-zero values as positive (push closer),
  while the original has a signed range centered around value 65.

---

## 4. Per-Scanline Z-Gradient Table

### Table: `DAT_00817710` at `0x00817710`

Three 24-byte entries defining how Z changes per scanline row. Used by both SHP
blitters (`FUN_004373b0` standard, `FUN_00437a10` extended) as a Bresenham-style
accumulator:

| Entry | Fields (6 × i32) | Behavior |
|-------|-------------------|----------|
| 0 | `1, 1, 1, 1, -1, 1` | Standard: Z decreases by 1 every row (rate 1.0/row) |
| 1 | `2, 3, 2, 3, -1, 1` | Moderate: Z decreases by 2 every 3 rows (rate 0.67/row) |
| 2 | `1, 3, 1, 3, +1, 0` | Buildings: Z increases by 1 every 3 rows (rate 0.33/row) |

The gradient type flows from `BuildingClass::DrawBody` (param_9 = 2) through
`TechnoClass::DrawSHP` to `CC_Draw_Shape` **param_11** (not param_10), then
to `FUN_004373b0` param_9 (at `ESP+0x78` after stack setup). Verified from
assembly at `0x00437415`: `LEA EDX,[EAX*0x8 + 0x817710]` where EAX comes
from `ESP+0x78` = CC param_11. CC param_10 carries z_height (separate value).
**Buildings pass gradient type 2.**

Entry 2 (used by buildings): going DOWN the sprite (top-to-bottom rendering),
Z **increases** (further from camera). This gives the roof (top scanlines)
**lower Z** (closer) and the base (bottom scanlines) **higher Z** (further).
Entry 2 also triggers a different Z initialization path in the blitter
(field[5]=0) that includes sprite height in the base Z formula.

### Blitter Dispatch Mechanism

The per-scanline loop in `SHP_StandardBlitter` calls the **Blitter object's**
vtable+4 method per scanline. The Blitter is selected by `Blitter_selector`
based on draw flags. `Blitter_ClipAndSetup` (`0x007bc040`) does NOT touch the
Blitter — it only clips rectangles and locks source/dest surfaces, returning
pixel pointers. The Blitter object stays in the outer loop and controls the
per-scanline rendering behavior (opaque, translucent, Z-aware, etc.).

### Accumulator Logic

```c
z_gradient_entry = &DAT_00817710[gradient_type * 6];
// Fields read by the blitter (verified from assembly at 0x004374fa):
z_increment = z_gradient_entry[2];   // field[2] at offset +0x08: per-row accumulator step
z_threshold = z_gradient_entry[3];   // field[3] at offset +0x0C: step threshold
z_step_dir  = z_gradient_entry[4];   // field[4] at offset +0x10: +1 or -1

// Initial accumulator depends on rendering path:
// field[5]=1 (entries 0,1): z_accum = 0 (set at 0x004374f2)
// field[5]=0 (entry 2):    z_accum = field[3] - (spriteHeight % step)
//   Special case: if z_accum == field[3] → z_accum = 0, base_z += step_dir

// Per scanline row (assembly at 0x00437921):
z_accum += z_increment;              // [ESP+0x28] = field[2]
if (z_accum >= z_threshold) {        // [ESP+0x2c] = field[3]
    z_accum -= z_threshold;
    base_z += z_step_dir;            // field[4] at [gradient_ptr + 0x10]
}
```

Note: field[0]==field[2] and field[1]==field[3] for all three gradient entries,
so older docs referencing field[0]/field[1] produce correct results numerically.
The code actually reads offsets +0x08 and +0x0C (fields 2 and 3).

### Status in Rust Engine

Not implemented. Could be approximated in the vertex/fragment shader by computing
a per-row Z offset from the fragment's Y position within the sprite.

---

## 5. VXL (Voxel) Z-Buffer Pipeline

VXL units (tanks, ships, etc.) do **NOT** directly access `g_ZBuffer`. They use a
two-stage pipeline where Z-buffer interaction happens only at blit time.

### Stage 1: Software 3D Rasterization

Voxel models are rasterized into two private 256×256-byte intermediate buffers:
- `g_VXL_VisibilityMap` (`0x00b2ff78`) — per-pixel palette color indices
- `g_VXL_DepthMap` (`0x00b1d5e0`) — per-pixel depth (mirror mode only)

Depth ordering between voxel sections uses **painter's algorithm**: sections are
bubble-sorted by minimum Z depth (float at box record +0x24), then drawn
front-to-back (normal) or back-to-front (mirror mode).

Mirror rasterizers (`0x00757120`) have per-pixel depth test within the VXL's own
depth buffer:
```c
if ((ushort)g_VXL_DepthMap[pixel] < (depth >> 8)) {
    g_VXL_DepthMap[pixel] = (byte)(depth >> 8);
    g_VXL_VisibilityMap[pixel] = voxel_color;
}
```
Non-mirror rasterizers simply overwrite — the section sort ensures correctness.

### Stage 2: Blit to Screen (same SHP blitter)

The 256×256 buffer is RLE-encoded and blitted via the **same blitter pipeline**
as SHP sprites:
- Cached path (`0x00707480`): → `SHP_ExtendedBlitter` (`0x00437a10`)
- Uncached path (`0x00706ed0`): → `SHP_StandardBlitter` (`0x004373b0`)

Z-mode flags are computed via `vtable+0x68` in `TechnoClass__Draw` (`0x00706640`),
identical to SHP objects. The gradient table applies during the blit phase.

**Exception — VXL turrets** (`0x00706bd0`): hardcoded flags `0x2001` (no Z-test,
no Z-write), z_height = 1000, gradient type = 0.

### Key VXL Data

| Address | Name | Purpose |
|---------|------|---------|
| `0x00b2ff78` | g_VXL_VisibilityMap | 256×256 intermediate color buffer |
| `0x00b1d5e0` | g_VXL_DepthMap | 256×256 intermediate depth (mirror only) |
| `0x00846840` | Rasterizer function table | 16 entries, dispatched by 4-bit flags |

---

## 6. Animation Z-Buffer Interaction

`AnimClass::DrawIt` (`0x00422ca0`) computes Z-height from art.ini keys:

```c
// Non-flat animations (Flat=false, gradient type 2):
z_height = YDrawOffset + ZAdjust - AdjustForZ() - 2;

// Flat animations (Flat=true, gradient type 0):
z_height = YDrawOffset + ZAdjust - AdjustForZ() - 3;
```

- `YDrawOffset`: `AnimTypeClass+0x344` (from art.ini `YDrawOffset=`)
- `ZAdjust`: `AnimTypeClass+0x348` (from art.ini `ZAdjust=`), instance copy at
  `AnimClass+0x100` (can be overridden per-instance)

### Z-mode flags

Animations use `flags | 0x2000` (part of the 0x3000 blitter mask). The actual
Z-read/write comes from bits 1–2:

| Condition | Z-flags | Notes |
|-----------|---------|-------|
| Default (no 0x119 flag) | `0x00` | No Z interaction |
| 0x119 set, Scorch=false | `0x04` | Z-WRITE only |
| 0x119 set, Scorch=true | `0x06` | Z-READ + WRITE |
| Translucent (DetailLevel based) | `0x02`/`0x04`/`0x06` | Varies by translucency % |

**Key differences from TechnoClass::DrawSHP:**
- Animations DO add `0x800` **unless bit 0 (shadow mode) is set**:
  `if ((flags & 1) == 0) { flags |= 0x800; }` — shadow draws skip 0x800
- Animations do **NOT** add `0x200` (no sprite centering)
- Non-flat animations use gradient type **2** (same as buildings: roof closer)
- Flat animations use gradient type **0** (standard: Z decreases 1/row)

---

## 7. Draw Order and Layer System

### Layer Rendering Order

Objects are sorted into 5 display layers. The layer array is at `DAT_008a0360`
(5 × `DynamicVectorClass<ObjectClass*>`, 24 bytes each, total 120 bytes).
`DAT_008a0364` is the buffer pointer of layer 0. Layer names verified from
string table at `0x0081da78`; name↔index conversion via `FUN_0048e050`/`FUN_0048e090`:

| Layer | Index | INI Name | Contents | Z-Buffer |
|-------|-------|----------|----------|----------|
| Underground | 0 | `Underground` | Tunnel locomotor effects | Varies |
| Surface | 1 | `Surface` | Below-ground-level objects (e.g. submerged subs) | Varies |
| Ground | 2 | `Ground` | Buildings, infantry, vehicles | Varies |
| Air | 3 | `Air` | Aircraft, projectiles | Varies |
| Top | 4 | `Top` | Parachutes, top-layer effects | Varies |

Z-buffer mode is set **per-object** in `TechnoClass::DrawSHP` based on the
`vtable+0x68` virtual call result (see Z-mode table in Overview), not per-layer.
Buildings (case 2/3) use Z-WRITE ONLY — they unconditionally draw and write Z
but do not test against existing Z values for the main body. Per-pixel Z-testing
for buildings comes only from the BUILDNGZ.SHA Z-shape overlay path.

After layer 2 (Ground), building turrets are drawn from `g_BuildingClass_Array`.

**Walls are NOT layer objects.** They are rendered as cell overlays via
`FUN_006d7560` → `FUN_00480350` → `FUN_00547cf0` (tile renderer).
`TacticalClass::Draw` (`FUN_006d3f50`) uses a two-phase rendering system
controlled by `param_4`: Phase 1 (`param_4 == 1`) for terrain, Phase 2
(`param_4 == 2`) for objects, Phase 3 (`param_4 == 3`) for both sequentially.

**Phase 1 — Terrain pass** (8 steps):

1. `FUN_006d2b60` — Z-buffer dirty rect clear
2. `FUN_006d3660` — Shroud edges and icons
3. `FUN_006d2de0` — Terrain shadows
4. `FUN_006d3470` — Base terrain cells
5. `FUN_006d3290` — Smudges and craters
6. `FUN_006d3ac0` — Building overlays
7. `FUN_006d3040` — Overlays (**walls, ore, other cell overlays**)
8. `FUN_006d3870` — Animations

**Phase 2 — Object pass:**

9. `FUN_006d8db0` — Layer object rendering (buildings, units, aircraft, etc.)

**Critical finding (verified from full assembly trace):** SHP sprite blitters
selected for buildings do **NOT** read or write the Z-buffer per pixel. The
blitter at offset 0x130 (selected when `0x800` + Z-write-only flags) performs
50% alpha blending but ignores all Z-buffer parameters.

Depth ordering between walls and buildings works as follows:

1. Wall pixels (Phase 1, step 7) write `wall_z` to Z-buffer via `TMP_TileBlitter`
   per-pixel Z-test (`pixel_z <= zbuffer`) — this is the **only** active Z-write
2. Building pixels (Phase 2) are drawn **unconditionally** — the blitter neither
   tests nor writes Z-buffer values. Buildings always overwrite walls.
3. Correct visual ordering relies on **screen-Y sorting** within Phase 2 and the
   **layer system** (5 layers rendered in order)

The BUILDNGZ.SHA Z-shape overlay data IS loaded and allocated per building draw,
but the selected blitter **ignores it**. The Z-writing blitter at offset 0x10c
(which would use BUILDNGZ data) is unreachable when `0x800` is set.

**Implication for our engine:** The original engine's depth ordering for buildings
is simpler than previously documented — it's painter's algorithm (draw order)
rather than per-pixel Z-buffer testing. Our GPU Z-buffer approach with BUILDNGZ
depth atlas may actually provide BETTER depth precision than the original engine.

---

## 8. Bridge Rendering and Z-Buffer Interaction (verified)

Bridges achieve correct depth ordering through a combination of heightLevel
manipulation, pre-computed Z-adjust fields, and overlay rendering with the
standard TMP_TileBlitter Z pipeline. There is NO special bridge Z-buffer
handling — bridges use the same per-pixel Z-test as all other terrain tiles.

### Bridge Height Model

Bridge deck cells have `heightLevel` (CellClass+0x11B) set to `ground_height + 4`
during map load. This +4 is applied in two ways:

1. **ApplyBridgeTile** (`0x0057b440`): Sets `cell.heightLevel = sub_tile_height + reference_ground_height`
2. **Map init** (`FUN_0059e740`): Adds +4 to heightLevel for cells whose IsoTileTypeIndex matches the bridge tile set: `cell.heightLevel += 4`

The constant +4 means bridge decks are always 4 height levels (60 pixels) above
the ground beneath them. `GetEffectiveHeight` (`0x00487d50`) uses the same formula
for unit positioning: `heightLevel + ((cell_flags >> 7) & 1) * 4`.

### Three Pre-Computed Z-Adjust Fields

`Cell_ComputeZAdjust` (`0x00484680`) computes three Z-adjust values per cell:

| Field | Offset | Formula | Used By |
|-------|--------|---------|---------|
| cellZAdjust_top | +0x10A | `base + gradient * heightLevel - offset` | Buildings, normal overlays |
| cellZAdjust | +0x10C | `(+0x10A * intensityFactor) >> 16` | TMP_TileBlitter (tile Z), normal overlay CC_Draw_Shape |
| cellZAdjust_bottom | +0x10E | `base + gradient * (heightLevel + 4) - offset`, scaled | **Bridge overlay body** |

The +4 in the `+0x10E` formula is hardcoded — every cell pre-computes a
"bridge-level" Z-adjust regardless of whether it actually has a bridge.
The gradient and offset come from theater-specific tables at offsets from
the Rules class instance (e.g., `+0x3544` gradient, `+0x3540` offset for
the default theater).

### Bridge Tile Rendering (TMP_TileBlitter Path)

Bridge terrain tiles are drawn via:
```
Tactical_layer_terrain_shadows → iso_to_screen → CellOverlay_TileDraw → TMP_TileBlitter
```

In `CellOverlay_TileDraw` (`0x00480350`):
- **Y position**: `screenY + heightLevel * -15` (shifts tile UP by 15px per level)
- **Z-enable**: Always `1` (Z-test + Z-write active)
- **heightLevel**: Raw `CellClass+0x11B` value (bridge cells already have +4 baked in)
- **cellZAdjust**: `CellClass+0x10C` (pre-computed, includes heightLevel contribution)

TMP_TileBlitter Z formula:
```c
base_z = (DefaultZ + YOrigin - screenY - spriteHeight) - (spriteHeight * cellZAdjust) / 2;
// Per pixel: if (z_shape_value + base_z <= zbuffer[pixel]) { write; }
```

Since bridge cells have heightLevel = ground+4, their `cellZAdjust` (+0x10C) is
larger, producing a LOWER base_z (closer to camera). This makes bridge tile pixels
occlude ground-level pixels in the Z-buffer.

### Bridge Overlay Rendering (SHP Path via CC_Draw_Shape)

Bridge overlays (deck surface graphics with SHP overlays, not TMP tiles) are
drawn through `CellClass__DrawOverlay_Body` (`0x0047f6a0`) and
`CellClass__DrawOverlay_Shadow` (`0x0047f510`), called from `FUN_004d1890`
(base terrain renderer, case 0x14).

**Bridge body** (when `cell_flags & 0x80` is set):
```c
effective_height = heightLevel + ((cell_flags >> 7) & 1) * 4;  // +4 for bridges
CC_Draw_Shape(shp, frame, &pos, clip_rect,
    0x4E00,                    // flags: Z-buffered, centered, palette
    0,
    effective_height * -15 - 2, // Y-adjust
    0,                          // gradient type 0
    cell.cellZAdjust_bottom,    // Z-height from +0x10E (bridge-level Z!)
    0, 0, 0, 0, 0);
```

**Bridge shadow**:
```c
CC_Draw_Shape(shp, shadow_frame, &pos, clip_rect,
    0x4601,                    // flags: Z-buffered, centered, palette, darken
    0,
    heightLevel * -15 - 2,     // Y-adjust (NO +4, draws at ground level)
    0,                          // gradient type 0
    1000,                       // Z-height = default (no special depth)
    0, 0, 0, 0, 0);
```

Key distinction: body uses `+0x10E` (bridge-level Z) while shadow uses Z=1000
(default terrain Z). Shadow renders at ground height, body renders elevated.

### Bridge Railing/Pavement Overlay

`FUN_004802a0` → `FUN_00547230` draws bridge railings/pavement in the overlay
pass (Phase 1 step 7). Called for every cell in `FUN_006d7c00` (Tactical_layer_overlays
inner iterator). Renders using `CC_Draw_Shape` with flag `0x4601` and Y-adjust
`heightLevel * -15 + 0x3A` (58 decimal, approximately 2 tiles up from the base).

### Depth Ordering Summary

Units and objects sort correctly relative to bridges because:

1. **Units ON bridge**: Their cell has `heightLevel = ground + 4`, so their screen-Y
   position (used for draw-order sorting in Phase 2) is shifted up by 60 pixels.
   They sort in front of bridge surface tiles which were Z-written in Phase 1.

2. **Units UNDER bridge**: Their cell heightLevel is the raw ground height.
   Bridge tile Z-values (from heightLevel+4) are LOWER (closer to camera),
   so the bridge terrain pixels occlude the ground beneath. Units at ground
   height have screen-Y positions that sort them behind the bridge body.

3. **Bridge shadow**: Drawn at ground-level Z (=1000, default), so it appears
   on the ground surface beneath the bridge without interfering with bridge
   body depth.

4. **No special rendering pass**: Bridges are NOT separated into a special
   rendering pass. They go through the same Phase 1 terrain pipeline as all
   other tiles, using the standard TMP_TileBlitter Z-test.

### DAT_00B0782C (g_BridgeZ_Offset)

Value: **0** at static analysis time (runtime-initialized).
This is used exclusively by `ShipLocomotionClass` for ships navigating under
bridges. It adjusts the Z-coordinate of ship destinations when the destination
cell has flag `0x100` (bridge structural cell), ensuring ships render at the
correct depth beneath bridges. It is NOT used by the rendering pipeline directly.

Xrefs:
- `ShipLocomotionClass__InitBridgeZOffset` (`0x0069ebd0`) — WRITE (sets the value)
- `FUN_0069f450` (`0x0069f450`) — READ (applies to ship Z when cell has bridge)
- `ShipLocomotionClass__Process_Drive_Track` (`0x006a0000`) — READ (2 locations)

---

## 9. Key Addresses

### Functions

| Address | Ghidra Label | Purpose |
|---------|-------------|---------|
| `0x00547cf0` | TMP_TileBlitter | 60×29 diamond renderer with TMP Z-data |
| `0x004aed70` | CC_Draw_Shape | Main SHP draw entry point (50+ call sites) |
| `0x004373b0` | SHP_StandardBlitter | Per-scanline loop with Z-gradient |
| `0x00437a10` | SHP_ExtendedBlitter | Shadow/Z-overlay variant |
| `0x0045e8f0` | LoadBuildingZShape | Loads BUILDNGZ.SHA, applies -65 remap |
| `0x00490b90` | Blitter_selector | Picks blitter from draw flags (164 lines) |
| `0x00490e50` | Blitter_selector_extended | Same for extended blitter path |
| `0x006d8db0` | Tactical_ObjectRenderingLoop | Iterates 5 display layers, calls Draw_It |
| `0x007bcf50` | ZBuffer_RectClear | Thin wrapper, calls virtual fill method |
| `0x007bcfb0` | ZBuffer_Clear | Row-by-row fill with 0xFFFF (the actual clear) |
| `0x007bd130` | ZBuffer_GetScanlinePtr | Gets pointer to Z-buffer row |
| `0x0043d290` | BuildingClass_DrawBody | Building body draw dispatch |
| `0x00705e00` | TechnoClass_DrawSHP | Shape draw with Z params |
| `0x00480350` | CellOverlay_TileDraw | Wall/overlay draw, calls tile renderer |
| `0x007bc970` | ZBuffer_Constructor | Creates 0x30-byte Z-buffer surface object |
| `0x006bb9a0` | WinMain | Creates Z-buffer, sets DefaultZ = 0x8000 |
| `0x00497100` | Blitter_ZClip_Plain16_WritesZ | Per-pixel Z-shape blitter: `base_z - z_shape` |
| `0x00495bc0` | Blitter_ZBuf_Intensity25pct_WritesZ | Per-pixel Z R+W with 25% blend |
| `0x0048ebf0` | Blitter_Init_All | Creates all blitter objects with vtables |
| `0x00456f80` | BuildingClass_AdjustZHeight | vtable+0x464: ±500 (threshold 1500) |
| `0x006d3f50` | TacticalClass_Draw | Two-phase renderer (1=terrain, 2=objects, 3=both) |
| `0x006d2b60` | Tactical_ZBufferDirtyClear | Phase 1 step 1: dirty rect Z-clear |
| `0x006d2de0` | Tactical_TerrainShadows | Phase 1 step 3: terrain shadows |
| `0x006d3470` | Tactical_BaseTerrainCells | Phase 1 step 4: base terrain cells |
| `0x006d3040` | Tactical_Overlays | Phase 1 step 7: walls, ore, cell overlays |
| `0x0069e900` | SHP_GetFrameCompressionFlag | Bit 1 of frame byte 8: std vs extended |
| `0x0069e7e0` | SHP_GetFrameRect | Returns frame x/y/w/h for given frame index |
| `0x0069e740` | SHP_GetFrameData | Returns decompressed frame pixel data ptr |
| `0x0069e580` | SHP_Resolve | Resolve SHP reference, ensure loaded |
| `0x007bc040` | Blitter_ClipAndSetup | Clip rects + configure source surface |
| `0x004114b0` | CircBuf_GetScanlinePtr | Generic circular buffer row pointer |
| `0x0043ad00` | PixelBuffer_Init | Init buffer descriptor, optional alloc |
| `0x0043ae50` | PixelBuffer_Free | Free if owned, zero descriptor |
| `0x005b40b0` | LoadFileFromMIX | Generic file loader from MIX archives |
| `0x00706640` | TechnoClass__Draw | Sets Z-mode flags for VXL (same as SHP) |
| `0x00706ed0` | TechnoClass__Render | VXL uncached → SHP_StandardBlitter |
| `0x00707480` | VXL_CacheBlit | VXL cached → SHP_ExtendedBlitter |
| `0x00706bd0` | VXL turret draw | Hardcoded `0x2001` (no Z-test/write) |
| `0x00754510` | VXL_Sort_Rasterize | Bubble-sort sections by Z, then rasterize |
| `0x00757120` | VXL_Rasterizer_Mirror | Per-pixel depth test in g_VXL_DepthMap |
| `0x00422ca0` | AnimClass::DrawIt | Anim draw with ZAdjust + gradient type 0 or 2 |
| `0x00484680` | Cell Z-adjust init | Computes cellZAdjust from heightLevel |

### Global Data

| Address | Type | Purpose |
|---------|------|---------|
| `DAT_00887644` | `ZBuffer*` | 16-bit per-pixel depth surface |
| `DAT_0087e8a4` | `ABuffer*` | 16-bit intensity/remap overlay surface |
| `DAT_0089ddbc` | `SHP*` | Loaded BUILDNGZ.SHA data |
| `DAT_00817710` | `int[3][6]` | Z-gradient parameter table |
| `DAT_008a0360` | `DynamicVectorClass[5]` | Display layer array (24 bytes/entry, base) |
| `DAT_008a0364` | `ObjectClass**` | Buffer pointer of layer 0 (inside first entry) |
| `DAT_0081da78` | `char*[5]` | Layer name string table (Underground/Surface/Ground/Air/Top) |
| `DAT_0081dc24` | `uint` | Blitter flag mask = `0x3000` (tests bits 12-13) |
| `DAT_00abb120` | `byte[29×60]` | Isometric diamond pixel offset table (runtime-initialized) |
| `DAT_00aa074c` | `byte[29×60]` | Isometric diamond screen offset table (runtime-initialized) |
| `DAT_00aa154c` | `byte[]` | Isometric diamond scanline width table (stride `0x6CC` = 60×29, runtime-initialized) |
| `0x00b2ff78` | `byte[256×256]` | g_VXL_VisibilityMap (intermediate voxel color buffer) |
| `0x00b1d5e0` | `byte[256×256]` | g_VXL_DepthMap (intermediate voxel depth, mirror only) |
| `0x00846840` | `func_ptr[16]` | VXL rasterizer function table (4-bit dispatch) |

---

## 10. Implementation Status in Rust Engine (updated 2026-03-22)

### How Our Engine Renders Depth

Our engine uses painter's algorithm (draw order) for sprite-vs-sprite layering,
matching gamemd.exe. The GPU depth buffer handles terrain occlusion only:

- **Terrain tiles** — `zdepth_shader.wgsl` samples per-pixel TMP Z-data from an R8
  depth atlas, writes `@builtin(frag_depth)`. Depth write ON. Matches the original's
  per-pixel terrain Z-write via TMP_TileBlitter.
- **Wall overlays** — `zdepth_shader.wgsl` with depth write ON (`LessEqual`). Walls
  write depth so sprites behind them are occluded. Matches gamemd.exe where walls
  write Z via TMP_TileBlitter in terrain pass step 7.
- **Non-wall overlays (ore, terrain objects)** — `zdepth_shader.wgsl` with depth
  write OFF. Read terrain depth for cliff occlusion only.
- **All sprites (buildings, units, infantry, damage fires)** — `zdepth_shader.wgsl`
  with depth write OFF (`LessEqual`). Sprites read terrain depth for cliff occlusion
  but don't write depth. Sprite-vs-sprite ordering is pure draw order.
- **Unified Y-sorted object pass** — VXL units and SHP entities are merged into a
  single Y-sorted draw pass via multi-way merge, matching gamemd.exe Layer 2 only
  if each class uses its native virtual `GetYSort` key. Base `ObjectClass::GetYSort`
  is `X + Y`; `AnimClass` adds `YSortAdjust`; `BuildingClass` adds `+32` for
  `BuildingType+0x16C5` and `-16` for `BuildingType+0x16B7`. No class/id
  tiebreakers were found in the 2026-05-28 YSort override census.
- **Building turrets** — drawn in a separate pass after all layer-2 objects, matching
  gamemd.exe's turret pass after the ground layer.
- **Damage fires** — Y-sorted with buildings in the object pass (not a separate
  terrain pass), matching gamemd.exe where FIRE anims are Layer 2 objects.
- **Cliff redraw** — terrain cliff tiles redrawn after entities with depth write ON
  so cliff pixels occlude sprites behind them.
- **Depth function** — single `compute_sprite_depth()` for all sprites. No per-type
  bias constants. Depth only determines terrain occlusion, not sprite ordering.

Source note: the per-class `GetYSort` details above come from
`docs/research/PERCLASS_VTABLE_B8_YSORT_OVERRIDE_CENSUS_GHIDRA_REPORT.md`.

### BUILDNGZ.SHA — Removed

BUILDNGZ.SHA is loaded but **not used** in the original engine (the Z-writing blitter
at offset 0x10c is unreachable when `0x800` flag is set). Our engine previously used
it for per-pixel building depth via the zdepth shader, but this caused edge artifacts
with walls. It has been completely removed from the rendering pipeline to match the
original's behavior.

### Draw Pass Order

```
1. Terrain          — zdepth pipeline (depth write ON)
2. Wall overlays    — passthrough (no depth test, painted over terrain)
3. Non-wall overlays — passthrough pipeline (depth compare Always, no test)
4. UNIFIED MERGE    — VXL + SHP + damage fires, Y-sorted, interleaved draw calls
5. Building turrets — zdepth overlay no-write (after all layer-2 objects)
6. Cliff redraw     — overlay pipeline (depth write ON)
7. Debug/fog/UI     — overlay pipeline
```

### Comparison with Original Engine

| Aspect | Original (gamemd.exe) | Our engine | Match? |
|--------|----------------------|------------|--------|
| Terrain per-pixel Z | TMP tile blitter, Z R+W | zdepth shader, frag_depth | Match |
| Wall Z | TMP blitter Z R+W (flag 0x02), but sprites ignore Z (0x800) | passthrough (no depth test — wall Z-writes only affect TMP rendering in original, not sprites) | Match |
| Non-wall overlay Z | TMP blitter skips Z (flag 0x02 clear) | passthrough pipeline (Always compare) | Match |
| SHP sprite Z | No Z interaction (0x800 flag) | passthrough (no depth test, painter's alg) | Match |
| Object sort | Layer 2 sorted by virtual `GetYSort`: base `X+Y`, plus AnimClass and BuildingClass overrides | depth-sorted (iso_row based), multi-way merge | Partial: missing proven per-class `GetYSort` deltas |
| Building turrets | Separate pass after layer 2 | Separate pass after merged objects | Match |
| Damage fires | AnimClass in Layer 2, Y-sorted | In SHP pass, Y-sorted with buildings | Match |
| Building sort key | `BuildingClass::GetYSort = ObjectClass::GetYSort` plus conditional `+32` / `-16` type flags | screen_y from foundation | Partial: missing conditional deltas |
| BUILDNGZ per-pixel | Loaded but ignored | Removed | Match |
| Cliff occlusion | No cliff-over-building Z test | Cliff redraw pass | Ours is better |
| Per-scanline gradient | 3-entry Bresenham table | Not implemented | Missing (cosmetic) |
| Shadows | Drawn before sprite per object | Not implemented | Missing |
| Flat anims | Terrain pass step 8 (below objects) | Mixed into SHP pass | Missing |
| Smudges | Terrain pass step 5 | Not implemented | Missing |

### Intentional Improvements Over Original
- **Cliff occlusion** — our cliff redraw pass provides correct building-behind-cliff
  rendering that the original lacks (original draws buildings over cliff terrain)
