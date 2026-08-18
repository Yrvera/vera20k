# Voxel (VXL) Rendering Pipeline in gamemd.exe — Complete Analysis

## Overview

The VXL rendering system is a **software 3D rasterizer** that transforms voxel models into 2D sprites, using **painter's algorithm depth sorting**, **pre-computed lighting LUTs**, and a **function-pointer dispatch table** for 16 rasterizer variants. It renders to a 256x256 pixel tile map, then blits the result to the screen. A **voxel cache** avoids re-rendering when the same facing/frame has already been drawn.

---

## 1. File Formats Involved

### VXL File (loaded at `0x00755DB0`)

The loader allocates a 0x1C-byte (28 bytes) control structure:

- **+0x04**: vertex count
- **+0x08**: section/limb count
- **+0x0C**: body data size
- **+0x10**: vertex buffer (`vertex_count * 12` bytes -- 3 floats per vertex)
- **+0x14**: section array (`section_count * 0xA4` bytes -- 164 bytes per section)
- **+0x18**: body data buffer (raw voxel span data)

Each **section record** (0xA4 = 164 bytes):

- **+0x0C**: unknown field
- **+0x10..0x3F**: 48-byte transform matrix (3x4 row-major, 12 floats)
- **+0x40..0x9F**: 8 AABB corner vertices (8 x 12 bytes) -- computed from min/max bounds as all 8 combinations of (min_x/max_x, min_y/max_y, min_z/max_z)
- **+0xA0**: R color tint
- **+0xA1**: G color tint
- **+0xA2**: B color tint
- **+0xA3**: Alpha (0 = fully transparent -- triggers transparent rasterizer path)

During loading, if an embedded palette+normals block is present, it reads 256 RGB entries (3 bytes each, 768 total) into the global palette at `DAT_00b2fb79`, then builds the remap table via `FUN_00758B70`.

### HVA File

Per-frame per-section 3x4 transform matrices. Indexed as `hva_transforms[(frame % num_frames) * section_count + section_index]`. Applied by multiplying with the facing rotation matrix via `FUN_005AF980`.

### VPL File (voxels.vpl)

Pre-computed lighting lookup -- loaded during game init at `0x0052ba60`. Contains `numSections * 256`-byte pages, mapping `table[brightness_page][color_index]` -> shaded palette index.

---

## 2. One-Time Initialization (Game Startup)

### Light Direction Setup (`FUN_00754C00` at `0x00754C00`)

- Pushes a matrix, rotates by the given angle
- Normalizes the vector `(-0.707, -0.707, 0)` (45 deg from X axis in XY plane)
- Stores the result as the global **light direction** at `DAT_00887470..78`

### Master Lighting Matrix Precomputation (`FUN_00754CB0` at `0x00754CB0`, 3290 bytes)

This is the **largest single initialization function** for voxels. It precomputes:

1. **24 camera view matrices** (8 angles x 3 zoom levels):
   - 8 angles: 45 deg, 135 deg, 225 deg, 315 deg (diagonal corners) + 0 deg, 90 deg, 180 deg, 270 deg (cardinal)
   - Two zoom distances from `DAT_00b43f08` and `DAT_00b44310`
   - Stored in `DAT_00b43f40` through `DAT_00b453c8` (48 bytes = 12 floats each)

2. **Quaternion representations** for each matrix (for smooth interpolation via slerp)

3. **256 precomputed normal direction vectors** at evenly spaced rotations (stored in `DAT_00b432d8` with tangent vectors in `DAT_00b444a0`)

4. **16 additional light-direction matrices** and **4 face-normal identity matrices** for box-face rendering

### Palette Remap Table (`FUN_00758B70` at `0x00758B70`, 665 bytes)

Builds a **256x32 byte lookup table** (8192 bytes):

- **32 brightness levels** split into two halves:
  - Levels 0-15 (dark): `brightness = level * scale * gain + offset` (linear ramp from shadow to mid)
  - Levels 16-31 (bright): `brightness = ((level-16) + level) * scale + bright_offset` (mid to highlight, clamped to 255)
- For each `(brightness_level, palette_color)`: multiplies the palette RGB by the brightness float, finds nearest matching palette index via Euclidean distance search (`FUN_00758EA0`)
- Result indexed as `table[(brightness << 8) | color_index]`

---

## 3. Per-Frame Rendering Pipeline

### Entry Point: `FUN_00706640` (TechnoClass::DrawVoxel, 262 lines)

The top-level drawing function for voxel units:

1. **Determines render flags** based on cloak state (vtable+0x68):
   - State 1: semi-transparent (0x2002)
   - State 2,3: more transparent (0x2004)
   - State 4: ghost/stealth (0x200A or 0x200C)
   - State 5: invisible -- early return

2. **Checks voxel cache** -- if a cached sprite exists for this facing, blits it via `FUN_00707480` and returns immediately

3. **Calls `FUN_00706ed0`** (the actual voxel rendering):
   - Copies the facing transform matrix (48 bytes = 12 floats)
   - Gets VXL and HVA data from the type class
   - Checks **mirror flag** at `typeclass+0xDAC`; if set, enables `DAT_00b43180 = 1`
   - Calls `FUN_00753D00` (init VXL with Blinn-Phong lighting)
   - Calls `FUN_00753E00` (clear tile map)
   - **For each section**, reads the HVA animation matrix for the current frame:
     `hva_matrix = hva_data[(frame % frame_count) * section_count + section] * 0x30 + hva_data_ptr`
   - Multiplies by facing rotation via `FUN_005AF980`
   - Calls `FUN_007540F0` (submit bounding box) for each section
   - Calls `FUN_00754510` (sort & rasterize) -- returns a 6-float screen rect
   - Generates the final surface via `FUN_004AF2A0` -> `FUN_004373B0`

4. **Updates dirty rect** for incremental redraw optimization
5. **Stores in cache** if not already cached

### Step A: VXL Lighting Initialization

Two quality levels:

**Simple ambient** -- `FUN_00758670` (for VoxelAnims via `FUN_00753C80`):

```
for each normal[i] in normal_table:
    brightness = dot(normal[i], light_dir)
    if brightness >= threshold:
        lut[i] = ftol(brightness * scale)
    else:
        lut[i] = 0  // shadow
ambient_rgb = (0x10, 0x10, 0x10)
```

**Blinn-Phong with specular** -- `FUN_007586F0` (for TechnoClass via `FUN_00753D00`):

```
halfway = normalize(light_dir + view_dir)
for each normal[i] in normal_table:
    diffuse = max(0, dot(normal[i], light_dir))
    specular = dot(normal[i], halfway)
    specular = specular / (shininess - specular * shininess + specular)  // Schlick approx
    if (diffuse + specular) >= threshold:
        lut[i] = ftol((diffuse + specular) * scale)
    else:
        lut[i] = 0
```

Both write a 256-byte brightness LUT at `DAT_00b45990`. This LUT, combined with the palette remap table, gives the final shaded color: `final_color = remap_table[(lut[normal_index] << 8) | voxel_color_index]`.

### Step B: Clear Tile Map (`FUN_00753E00`)

- Resets box count (`DAT_00b2d820`) and quad count (`DAT_00b2fb70`) to 0
- Manages **two 256x256-byte buffers**:
  - `DAT_00b2ff78`: visibility bitmap
  - `DAT_00b1d5e0`: depth bitmap (used only in mirror mode)
- **First frame**: clears all 64KB of each buffer
- **Subsequent frames**: only clears the rectangular region from the previous frame's dirty rect (`DAT_00b2fb60..6c`) -- a significant optimization
- Resets global AABB: min = +10000.0, max = -10000.0

### Step C: Submit Geometry

**Submit bounding box** -- `FUN_007540F0` (for each VXL section):

- Gets section data via `FUN_007564B0`: `sections_base + (vertex_offsets[section_idx] + sub_idx) * 0xA4`
- Transforms all **8 AABB corners** (from section +0x40) through the matrix stack via `FUN_005afb80`
- **Negates Y** (Y-down screen space convention)
- Tracks per-box local AABB + the corner with minimum Z (nearest to camera)
- Updates global AABB
- Stores into per-box records at stride **0x88 (136 bytes)** from `DAT_00b2d958`
- Depth sort key stored at offset +0x24 in each record

**Submit billboard quad** -- `FUN_00753F90` (for 2D flat sections):

- Transforms 4 corners, offsets by camera position, negates Y
- Updates global AABB
- Per-quad records at stride 0x48 (72 bytes) from `DAT_00b3ff78`

### Step D: Sort and Rasterize (`FUN_007542F0`, 111 lines)

```
1. center = (global_aabb_min + global_aabb_max) * 0.5

2. Process billboard quads (if any) via FUN_00756860

3. Build indirection array: pointers to box records -> DAT_00b2fe78

4. BUBBLE SORT by float at record[+0x24] (minimum Z depth)
   // O(n^2) but VXL section counts are small (typically 1-3 per unit)

5. Draw each sorted section:
   if (mirror_flag == 0):
       draw front-to-back (0 -> N)
   else:
       draw back-to-front (N -> 0)

   for each section: call FUN_00756590()

6. Compute screen bounding rect centered at (128, 128) with +8/+4 padding
7. Save dirty rect for next frame's partial clear
```

### Step E: Section Rasterization (`FUN_00756590`, 74 lines)

For each section:

1. **Resolve section data pointer**: `sections_base + (vertex_offsets[param2+4] + param2+8) * 0xA4`
2. **Read per-section RGBA** from +0xA0..0xA3
3. **Read draw parameters** from global table at `DAT_008468xx` (stride 0x24 = 36 bytes):
   - Flags at +0x00, dimensions at +0x04..0x0C, offsets at +0x14..0x1C
4. **Project to integer screen coords** (9-12 `ftol` calls)
5. **Build rasterizer flags** (4 bits):
   - Bit 0: from draw params table flags
   - Bit 1: mirror mode (`DAT_00b43180`)
   - Bit 2: alternate rendering mode (`DAT_00b43184`)
   - Bit 3: section alpha == 0 (fully transparent)
6. **Dispatch to rasterizer** via function table at `PTR_FUN_00846840[flags]`

### Rasterizer Function Table (16 entries at `0x00846840`)

| Index | Flags | Address | Description |
|-------|-------|---------|-------------|
| 0 | `----` | `0x007DF7C0` | Standard opaque |
| 1 | `---M` | `0x007DF8C0` | Mirrored opaque |
| 2 | `--A-` | `0x00757120` | Alternate mode opaque |
| 3 | `--AM` | `0x00757360` | Alternate mode + mirror |
| 4 | `-B--` | `0x007DF9C0` | Mode B opaque |
| 5 | `-B-M` | `0x007DFAE0` | Mode B + mirror |
| 6 | `-BA-` | `0x00757980` | Mode B + alternate |
| 7 | `-BAM` | `0x00757BF0` | Mode B + alternate + mirror |
| 8 | `T---` | `0x007DFC00` | Transparent |
| 9 | `T--M` | `0x007DFD00` | Transparent + mirror |
| 10 | `T-A-` | `0x007581F0` | Transparent + alternate |
| 11 | `T-AM` | `0x00758430` | Transparent + alternate + mirror |
| 12-15 | (same as 8-11) | | Duplicates of transparent variants |

The rasterizers at `0x007DF7C0`-`0x007DFAE0` are in a different code section (likely optimized/hand-written assembly), while `0x00757120`-`0x00758430` are in the main voxel code section.

---

## 4. Smooth Facing Interpolation

### Precomputed Matrix Lookup (`FUN_007559B0`)

- Direct lookup: copies 48-byte matrix from `DAT_00b45188 + facing_index * 0x30`
- Called from targeting, locomotion, weapon rendering (8 callers)

### Interpolated Matrix (`FUN_00755A40`)

- If `source_facing == target_facing`: returns precomputed matrix directly
- Otherwise: **quaternion slerp** between the two facing quaternions (`FUN_00646590`), then converts back to matrix (`FUN_00646980`)
- This creates the smooth turning animation seen on voxel units

### VXL Turret Tilt (`FUN_00729B40`, 754 bytes)

Computes a 3x4 transform matrix based on locomotion state:

- **State 0 (idle)**: reads slope type from the cell at +0x11C; computes tilt from terrain slope
- **State 2 (takeoff)**: tilt = `(elapsed/total) * pi/2` -- smooth nose-up
- **State 3 (hover)**: fixed tilt = pi/2 (90 deg -- fully vertical, e.g., VTOL)
- **State 5 (diving)**: fixed tilt = -pi/2
- **State 6 (landing)**: tilt = `(1 - elapsed/total) * -pi/2` -- smooth nose-down to level
- **State 7 (similar)**: uses alternate constant multiplier
- Creates the characteristic nose-up/nose-down effect on aircraft

---

## 5. Voxel Cache System

The cache (`FUN_00706640` / `FUN_00707480`) avoids re-rendering:

1. Before rendering, checks if a cached bitmap exists for `param_3` (facing hash)
2. **Cache hit** (`FUN_00707480`): reads cached sprite dimensions (4 shorts: x, y, w, h), allocates a temp surface, blits the cached data with the appropriate shader effects (cloak, palette remap), updates dirty rect
3. **Cache miss**: renders via the full pipeline, then stores result in cache for future frames
4. Cache can be disabled via the INI key `DisableVoxelCache` (read in `TechnoTypeClass__ReadINI`)

---

## 6. VoxelDebris Rendering (`FUN_00748AF0`)

VoxelDebris (flying chunks from destroyed units) uses a **tile-based compressed sprite renderer**:

- Command stream of 16-bit words:
  - Bits 0-12: tile index (offset into tile data at 32-byte granularity)
  - Bits 13-15: command type:
    - `0x0000`: opaque blit -- copies 4x2 pixels directly
    - `0x2000`: transparent blit -- copies 4x2 pixels, skips pixels with bit 0x8000 set
    - `0x4000`: skip -- advances by 4 transparent pixels
- This is NOT the full 3D VXL rasterizer -- debris uses pre-rendered tile strips

---

## 7. Key Global State

| Address | Name | Purpose |
|---------|------|---------|
| `DAT_00b2d928` | VXL draw state | AABB, sort pointers, counters |
| `DAT_00b2d820` | box_count | Number of submitted VXL sections |
| `DAT_00b2fb70` | quad_count | Number of submitted billboard quads |
| `DAT_00b2d958` | box_records | Per-box data array (stride 0x88) |
| `DAT_00b3ff78` | quad_records | Per-quad data array (stride 0x48) |
| `DAT_00b2fe78` | sort_indices | Indirection array for depth sort |
| `DAT_00b2ff78` | visibility_map | 256x256 visibility bitmap |
| `DAT_00b1d5e0` | depth_map | 256x256 depth bitmap (mirror mode) |
| `DAT_00b43180` | mirror_flag | Enables mirrored rendering |
| `DAT_00b43184` | render_mode | Alternate rendering mode |
| `DAT_00b45990` | normal_lut | 256-byte per-normal brightness LUT |
| `DAT_00b45a8d..8f` | ambient_rgb | Ambient light color (default 0x10 each) |
| `DAT_00887470..78` | light_dir | Light direction vector (3 floats) |
| `DAT_00b43f40..b453c8` | view_matrices | 24 precomputed camera matrices |
| `DAT_00b45188` | facing_matrices | Precomputed facing rotation matrices |
| `DAT_00b432d8` | normal_vectors | 256 precomputed normal directions |
| `DAT_008467e0` | first_frame_flag | 0 after first clear (dirty rect optimization) |

---

## 8. Key Addresses Summary

| Address | Function | Role |
|---------|----------|------|
| `0x00754CB0` | Master lighting init | Precomputes all matrices, normals, quaternions |
| `0x00754C00` | Light direction setup | Sets global light vector |
| `0x00753D00` | Init VXL (Blinn-Phong) | Matrix + specular lighting setup per draw |
| `0x00753C80` | Init VXL (simple) | Matrix + ambient N*L lighting |
| `0x00753E00` | Clear tile map | Reset buffers + dirty rect optimization |
| `0x007540F0` | Submit bounding box | Transform + submit section geometry |
| `0x00753F90` | Submit billboard quad | Transform + submit 2D quad |
| `0x007542F0` | Sort and rasterize | Depth sort + dispatch to rasterizers |
| `0x00756590` | Section rasterizer | Per-section projection + rasterizer dispatch |
| `0x00756860` | Quad rasterizer | Billboard quad rendering |
| `0x00755DB0` | Load VXL file | Parse binary VXL from stream |
| `0x00758670` | Simple N*L lighting | Ambient-only normal brightness |
| `0x007586F0` | Blinn-Phong lighting | Diffuse + specular normal brightness |
| `0x00758B70` | Build remap table | 256x32 palette brightness LUT |
| `0x00758EA0` | Nearest color match | Euclidean distance palette search |
| `0x007559B0` | Get facing matrix | Precomputed matrix by index |
| `0x00755A40` | Interpolated facing | Quaternion slerp between facings |
| `0x00729B40` | Turret tilt matrix | Locomotion state -> tilt transform |
| `0x00706640` | TechnoClass draw | Top-level voxel draw + cache |
| `0x00706ED0` | TechnoClass render | Core rendering pipeline call |
| `0x00707480` | Cache blit | Draw from cached sprite |
| `0x0046B0C0` | VoxelAnim draw | Simpler draw path for voxel anims |
| `0x00846840` | Rasterizer table | 16 function pointers for rasterizer variants |
