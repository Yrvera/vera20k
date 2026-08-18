# VXL + HVA File Format & Consumer Pipeline — Ghidra Research Report

**Primary addresses:** `VXL_Load_File @ 0x00755DB0`, `HVA_Load_File @ 0x005BD5C0`,
`TechnoClass::Render @ 0x00706ED0`, `VXL_Section_Rasterizer @ 0x00756590`.
**Confidence:** HIGH overall. Every byte-layout claim is verified from disassembly
or memory read. A small number of consumer-side details are flagged INFERRED.
**Active in YR:** Yes — these systems run in every match that contains a vehicle,
ship, aircraft, voxel projectile, or voxel-bodied building.

This document covers the **on-disk byte layout** of `.vxl`, `.hva`, and `.vpl` files,
the **loaders** that ingest them, the **per-section composition pipeline** that
transforms them into draw calls, the **lighting / VPL palette path**, and the
**TS-legacy register** of dead-but-shipping code in the voxel pipeline. It is the
result of a planned investigation against the 35-function inventory in
`docs/plans/2026-05-10-vxl-hva-file-format-investigation-plan.md`.

The motivating gap was that four pre-existing reports
(`VOXEL_RENDERING_ANALYSIS.md`, `VOXEL_SLOPE_TILT_SYSTEM.md`,
`VXL_DRAW_MATRIX_GHIDRA_REPORT.md`, `VOXELANIMCLASS_GHIDRA_REPORT.md`) covered the
runtime rendering and matrix math, but **none specified the on-disk byte layout or
the loader semantics**. That gap is now closed.

---

## 1. Overview

A voxel asset in YR is a pair: a `.vxl` file (geometry + per-color normals) and a
`.hva` file (per-frame, per-section transformation matrices). Both are loaded
through the `CCFileClass` interface, allocated as a small heap struct that points
at variable-length data blocks, and consumed by a software rasterizer that walks
each "limb" (= section), applies the unit's body matrix × the HVA section matrix,
splats voxels into a 256×256 visibility/depth map, and finally blits the result
to the tactical surface. The `.vpl` file is a third asset that maps
`(lighting_index, color_index)` → final palette byte, used during raster.

The four parity-critical findings of this investigation:

1. **Section names in HVA are read off disk and immediately seeked past.** The
   engine pairs VXL limbs to HVA sections by **integer index**, never by name.
2. **The trailing `dup_count` byte at the end of every voxel run is consumed but
   never read.** Treat it as opaque padding.
3. **Specular strength is `3.0`, not `3.4`.** The Rust renderer's current value
   is wrong by enough to be visible on every voxel unit's highlight.
4. **The RA2 normals table (mode 4) has 245 entries, not 256.** Entries 245–249
   are byte-duplicates of entry 244; entries 250–255 do not exist in the binary.

All four would compound into a "subtly wrong" voxel render even with otherwise
correct math.

---

## 2. VXL File — On-Disk Byte Layout

Verified by decompiling and disassembling `VXL_Load_File @ 0x00755DB0`.

### 2.1 File header — 32 bytes

```
+0x00  ┌────────────────────────────────────────┐
       │ magic[16]  "Voxel Animation\0"         │  read into stack, NOT validated
+0x10  ├────────────────────────────────────────┤
       │ palette_count  (u32, LE)               │  governs palette section size
+0x14  ├────────────────────────────────────────┤
       │ limb_count  (u32, LE)                  │  → state +0x4
+0x18  ├────────────────────────────────────────┤
       │ tailer_count  (u32, LE)                │  → state +0x8
+0x1C  ├────────────────────────────────────────┤
       │ body_size  (u32, LE)                   │  → state +0xC
+0x20  └────────────────────────────────────────┘
```

**Verified.** The loader reads exactly 32 bytes (`Read(buf, 0x20)`), does **not**
compare the magic to anything, and stores the four trailing u32s into the VXL
state struct. Every internal byte is little-endian.

### 2.2 Palette section — `palette_count × 770` bytes (variable)

Each palette page on disk is `1 prefix + 768 RGB + 1 suffix = 770` bytes.

Two code paths gate the palette section:

- **Skip path (always taken in YR):** the loader calls
  `Seek(palette_count × 0x302, 1)` (relative seek by `palette_count × 770`) and
  proceeds to limb headers.
- **Full-read path:** reads all palette bytes into the global palette workspace,
  then calls `VXL_BuildRemapTable` (0x00758B70) — this is the
  `VXL_NearestColorMatch` (0x00758EA0) path. **Dead in YR.** Every caller of
  `VXL_Load_File` (the wrapper at `FUN_00755CD0`, all six
  `CDFileClass__Constructor` voxel sites, and `VoxelAnimTypeClass::ReadINI`)
  passes the `iStack_4 == 0` argument, taking the skip path. The full-read
  branch (~150 bytes at `0x00755ED6..0x00756034`) is shipping-but-unreachable.

> **Critical disparity.** Rust's `VXL_HEADER_SIZE = 802` is wrong: 802 = 32 (real
> header) + 770 (one palette page). Rust must read a 32-byte header, then seek
> past `palette_count × 770` bytes, then proceed to limb headers. Hard-coding
> 802 only works for files with `palette_count == 1`, and silently breaks for
> `palette_count > 1` or `palette_count == 0`.

### 2.3 Limb header — 28 bytes per limb, `limb_count` total

```
+0x00  ┌────────────────────────────────────────┐
       │ name[16]  (ASCII, zero-padded)         │  read, then DISCARDED
+0x10  ├────────────────────────────────────────┤
       │ limb_number  (u32)                     │  → heap +0x0  (used as tailer index)
+0x14  ├────────────────────────────────────────┤
       │ unknown_dword  (u32)                   │  → heap +0x4
+0x18  ├────────────────────────────────────────┤
       │ unknown_byte   (u8)                    │  → heap +0x8
+0x19  ├────────────────────────────────────────┤
       │ 3 bytes  (read, discarded)             │
+0x1C  └────────────────────────────────────────┘
```

**Verified.** The loader reads exactly 28 bytes per limb. Only `limb_number`,
`unknown_dword`, and `unknown_byte` are copied to the heap; the limb name and
the trailing 3 bytes are dropped. The on-heap stride is **12 bytes**, not 28.

The limb name is **not used by the runtime** for matching. Limbs reach their
tailer through the integer `limb_number` (used as an index into the tailer
array). Rust may store the name for diagnostics; it must not gate any logic on
it.

### 2.4 Body data — one contiguous block of `body_size` bytes

The body is read as a single `Read(body_buffer, body_size)` call. Its internal
structure (decoded by the rasterizer, not the loader) is:

```
body buffer (body_size bytes total, base = b)
  ┌────────────────────────────────────────┐
  │ span_start[size_x × size_y]  (i32 × N) │  -1 = empty column;
  │                                        │  ≥ 0 = byte offset into span_data
  ├────────────────────────────────────────┤
  │ span_end[size_x × size_y]    (i32 × N) │  one-past-end byte offset per column
  │                                        │  NOT consulted by the rasterizer
  ├────────────────────────────────────────┤
  │ span_data: variable-length runs        │
  │   per column starting at b + span_start[col]:
  │     [skip:u8][count:u8]                │
  │     [color:u8 normal:u8] × count       │
  │     [trailer:u8]   ← consumed, NEVER read
  │   ... repeat until the column's runs   │
  │   cover [0, size_z)                    │
  └────────────────────────────────────────┘
```

The three pointers (`span_start`, `span_end`, `span_data`) are NOT in the body
header — they are stored in each tailer (see §2.5). On-disk they are
**byte offsets relative to the body buffer base**; in memory the loader rebases
them to absolute pointers.

Column ordering is `col_idx = y × size_x + x` (Y outer, X inner). Verified from
`FUN_007DF7C0` and the orientation table at `0x008468C0`.

### 2.5 Tailer — 92 bytes per tailer on disk, expanded to 0xA4 (164) bytes in memory

```
File offset  Size  Field                In-memory offset
─────────────────────────────────────────────────────────────
+0x00        4     span_start_offset    +0x00 (REBASED to absolute pointer)
+0x04        4     span_end_offset      +0x04 (REBASED)
+0x08        4     span_data_offset     +0x08 (REBASED)
+0x0C        4     scale (f32)          +0x0C
+0x10       48     transform[12] (f32)  +0x10  (3×4 row-major; identity-copied)
+0x40       12     min_bounds[3] (f32)  expanded into 8 corners +0x40..+0x9F
+0x4C       12     max_bounds[3] (f32)  (12 bytes × 8 corners = 96 in-memory bytes)
+0x58        1     size_x (u8)          +0xA0
+0x59        1     size_y (u8)          +0xA1
+0x5A        1     size_z (u8)          +0xA2
+0x5B        1     normals_mode (u8)    +0xA3
─────────────────────────────────────────────────────────────
File total: 92 bytes        Memory total: 164 (0xA4) bytes
```

**Verified.** The on-disk size is exactly 0x5C (92). The in-memory tailer is
larger because the 24 bytes of `min[3] + max[3]` bounds are expanded at load
time into **8 OBB corner triplets** (one per corner of the axis-aligned box),
each 12 bytes, total 96 in-memory bytes. The corners are computed from a
hard-coded permutation of (min/max)x × (min/max)y × (min/max)z.

`transform[12]` passes through `FUN_005ae5e0`, which is an **identity 48-byte
memcpy** (verified by disassembling 0x005AE5E0: 12 dwords, no permutation, no
sign flip, no transpose). Therefore the on-disk layout equals the in-memory
layout, byte for byte. The "row-major, translation in indices [3, 7, 11]"
convention used by Rust matches the format gamemd reads off disk and the
format every downstream consumer (`VXL_Section_Rasterizer`,
`Locomotion_Matrix`, `VXL_GetFacingMatrix`) operates on.

> **Disparity (corrected).** Rust previously labeled `+0x0C` of the tailer as a
> "limb identifier u32". It is `f32 scale` — verified by the FLD float load at
> `0x007561a5`.

> **TS-legacy smell.** The 8-corner expansion at `+0x40..+0x9F` is a
> performance optimization (avoids re-computing corners every frame). Rust may
> store min/max only and lazily compute, with no observable difference. Match
> only the data needed — not the storage shape.

### 2.6 Disparity table for `src/assets/vxl_file.rs` and `src/assets/vxl_decode.rs`

| Rust constant / behavior | Rust value | Binary truth | Severity |
|---|---|---|---|
| `VXL_MAGIC` validation | "Voxel Animation\0" enforced | Read but **not validated** | OK to keep enforcement |
| `VXL_HEADER_SIZE = 802` | 802 | **32 bytes header + variable palette section** | **HIGH — wrong for any non-1-page palette** |
| Palette skip | not implemented in current code path | `Seek(palette_count × 770, 1)` between header and limbs | **HIGH if missing** |
| `SECTION_HEADER_SIZE = 28` | 28 | 28 (0x1C) | OK |
| `SECTION_TAILER_SIZE = 92` | 92 | 92 (0x5C) | OK |
| Tailer +0x0C labeled "limb_identifier u32" | u32 | **`f32 scale`** | MEDIUM — wrong field, wrong type |
| Limb name (file +0x00..+0x0F) used | depends | **never used at runtime** by gamemd | LOW — store optional |
| `dup_count` byte interpretation | "validation/padding — unused by us" | Consumed, **never read** by any rasterizer (verified across `0x7DF7C0`, `0x757120`, `0x7DF8C0`) | **VERIFIED — Rust comment is correct; do not validate equality with `count`** |
| `span_end_offset` | stored | Stored on heap but **never consulted** by the rasterizer; only `span_start` is read | LOW — Rust may store or skip |
| Column iteration order | `y × size_x + x` | `y × size_x + x` | OK |
| `span_start[col] == -1` | empty-column skip | empty-column skip (`if (-1 < iVar2)`) | OK |

---

## 3. HVA File — On-Disk Byte Layout

Verified by decompiling and disassembling `HVA_Load_File @ 0x005BD5C0`.

### 3.1 In-memory struct — 16 bytes

```
+0x00  byte   status            (1 = load failed; set by wrapper FUN_005bd570)
+0x04  u32    section_count     (loaded from disk +0x14)
+0x08  u32    frame_count       (loaded from disk +0x10)
+0x0C  void*  matrix_buffer     (operator_new(frame × section × 0x30))
```

The HVA struct has **no other fields**; the destructor `FUN_005bd5a0` only frees
`+0x0C`. **Section names are NOT stored in memory** at all.

### 3.2 File header — 24 bytes

```
+0x00  ┌────────────────────────────────────────┐
       │ filename[16]  (ASCII, zero-padded)     │  read, NOT validated, NOT stored
+0x10  ├────────────────────────────────────────┤
       │ frame_count  (u32, LE)                 │  → struct +0x8
+0x14  ├────────────────────────────────────────┤
       │ section_count  (u32, LE)               │  → struct +0x4
+0x18  └────────────────────────────────────────┘
```

**Verified.** The loader reads exactly 24 bytes (`Read(buf, 0x18)`). Disassembly
at `0x005BD60D-0x005BD61A` confirms: `[ESP+0x28] (= buf+0x14)` → struct +0x4,
`[ESP+0x24] (= buf+0x10)` → struct +0x8. So **frame_count is at file offset
0x10** and **section_count is at file offset 0x14**. The in-memory struct
swaps them (section in +0x4, frame in +0x8); this is internal-only.

### 3.3 Section name table — 16 × `section_count` bytes — **SEEKED PAST**

```
0x005BD668   MOV ECX, [EBX+0x4]    ; section_count
0x005BD66B   MOV EAX, [ESI]
0x005BD66D   SHL ECX, 0x4          ; ECX = section_count * 16
0x005BD670   PUSH 0x1              ; SEEK_CUR
0x005BD672   PUSH ECX
0x005BD675   CALL [EAX+0x28]       ; CCFile::Seek(section_count * 16, 1)
```

**The section names are read off disk by gamemd and immediately discarded.**
The pairing of HVA sections to VXL limbs is **purely positional**: section index
*i* corresponds to limb at array index *i*. There is no name-based lookup
anywhere in the runtime draw pipeline.

> **Behavioral disparity.** Rust currently reads and stores the section names.
> This is not a *correctness* disparity (both reach the same matrix offsets and
> produce the same render), but Rust holds extra in-memory data that gamemd
> discards. Memory cost is negligible. **Decision: keep storing names in Rust
> for diagnostics. Do not gate any logic on them.**

### 3.4 Matrix block — `frame_count × section_count × 48` bytes, frame-major

```
For frame_idx in 0 .. frame_count:
  For section_idx in 0 .. section_count:
    Read 48 bytes (12 × f32) → matrix_buffer[frame_idx × section_count + section_idx]
```

Verified from disassembly at `0x005BD68C-0x005BD6E8`:

- Outer loop (`[ESP+0x10]`) iterates `0..frame_count` (struct +0x8).
- Inner loop (`EBP`) iterates `0..section_count` (struct +0x4).
- Index = `section_count × frame_idx + section_idx` (frame-major contiguous).
- Each matrix passes through `FUN_005ae5e0` (identity memcpy, see §2.5) before
  storage.

So **the on-disk byte layout of HVA matrices is identical to the in-memory
byte layout, with no transposition or reordering**. The same 12-float
row-major convention used for VXL tailer transforms applies here.

### 3.5 Edge cases (verified)

| Case | Behavior |
|---|---|
| `section_count == 0` AND `frame_count == 0` | `operator_new(0)` returns a unique non-null pointer (MSVC). Both loops are skipped. Returns 1 (success). Result: HVA with `matrix_buffer != NULL` but zero-size. Rust's `InvalidHvaFile` rejection is stricter — minor disparity, harmless because no real HVA has zero of either. |
| `operator_new` returns NULL | Error path: closes file, returns 0. `matrix_buffer` is left at 0. Wrapper sets `status = 1`. |
| Per-matrix Read returns < 48 bytes | Error path: closes file, frees `matrix_buffer`, sets it to 0, returns 0. Wrapper sets `status = 1`. **Note**: status byte `+0x0` is NOT modified inside the loader; the wrapper at `FUN_005bd570` is responsible. |

### 3.6 Disparity table for `src/assets/hva_file.rs`

| Rust assumption | Binary truth | Verdict |
|---|---|---|
| `HVA_MIN_SIZE = 24` | Header is exactly 0x18 | OK |
| `SECTION_NAME_SIZE = 16` | `section_count × 16` skip | OK |
| `frame_count` at file offset 16 | Buffer `+0x10` → struct +0x8 → outer loop bound | OK |
| `section_count` at file offset 20 | Buffer `+0x14` → struct +0x4 → inner loop bound | OK |
| Matrix is 3×4 row-major, 12 f32, translation at indices `[3,7,11]` | `FUN_005ae5e0` identity-copies; consumers index as row-major | OK |
| Matrix index = `frame × section_count + section` | `section_count × frame + section` (algebraically identical) | OK |
| Section names read and stored | Read and **seeked past** | LOW — Rust over-reads but lands at the same matrix offset |
| `section_count == 0` returns InvalidHvaFile | gamemd accepts | LOW — defensible defensiveness |

---

## 4. VPL File — On-Disk Byte Layout & Page Indexing

Verified by decompiling `FUN_00753B70` (VPL load wrapper), `FUN_00758950`
(workspace allocator), `FUN_00758A30` (VPL file read), and the rasterizer at
`VXL_Rasterizer_RenderMode @ 0x007DF9C0`.

### 4.1 File header — 16 bytes

```
+0x00  u32   header[0]          (modding lore: "first_remap" — purpose unverified)
+0x04  u32   header[1]          ("last_remap" — purpose unverified)
+0x08  u32   page_count         (used to size the page-data read)
+0x0C  u32   header[3]          (unknown — possibly version/format flag)
```

### 4.2 Body

```
+0x10           palette: 768 bytes (256 RGB triplets, 6-bit components)
+0x10 + 768    pages:    page_count × 256 bytes
                         (each page is a 256-byte palette LUT)
```

Workspace allocation is fixed at **32 KB** (`operator_new(0x8000)`), so the
effective max `page_count` is 128. Real `voxels.vpl` ships with 32 pages.

### 4.3 Page selection during raster

```
output_pixel_byte = vpl_pages[(g_VXL_NormalLUT[normal_idx] << 8) | color_idx]
```

i.e., **`page_idx = g_VXL_NormalLUT[normal_idx]` directly**. There is **no
clamp** in the binary — the lighting math is trusted to produce a valid byte.

`g_VXL_NormalLUT` lives at `0x00B45990` (256 bytes). It is filled by either
`VXL_BlinnPhongLighting` (for vehicles/buildings/turrets) or
`VXL_SimpleLighting` (for VoxelAnims). See §6.

> **Disparity.** Rust's `VplFile::get_palette_index` clamps `page.min(num_sections-1)`.
> The binary does not. For stock `voxels.vpl` the clamp is a no-op (page byte
> always falls within range). For modded VPLs with too few pages, behaviors
> diverge. **Decision: keep the Rust clamp as a defensive deviation; document it.**

---

## 5. Per-Section Composition Pipeline

Verified by decompiling `TechnoClass::Render @ 0x00706ED0`, the three locomotor
draw-matrix functions, `BuildVXLTurretMatrix @ 0x00458810`, and
`VXL_Section_Rasterizer @ 0x00756590`.

### 5.1 The unit voxel pipeline (Drive / Ship / Fly)

```
A) Locomotor body matrix
   DriveLocomotionClass::Draw_Matrix  (0x004AFF60)
   ShipLocomotionClass::Draw_Matrix   (0x0069F670)  [byte-identical to Drive]
   FlyLocomotionClass::Render_Matrix  (0x004CFB00)
     produces body_matrix = facing_rot × shear × Rx_roll × Ry_pitch × slope
     (Rx/Ry only on tilt path; flat slope=0 → facing_rot only)

B) TechnoClass::Render  (0x00706ED0)
   1. local_c0 ← body_matrix
   2. local_90 ← body_matrix
   3. VXL_Init_BlinnPhong(0, local_90, voxel_palette, light_dir, shininess=3.0)
   4. VXL_Clear_TileMap()
   5. for section_idx in 0 .. vxl.limb_count:
        if HVA != NULL:
          frame = (FootClass+0x538) % HVA->frame_count
          hva_mat = HVA->matrix_buffer + (frame × section_count + section_idx) × 0x30
          local_c0 = Locomotion_Matrix(local_c0, hva_mat)   // body × HVA
        composed = Locomotion_Matrix(local_c0, ???)
        VXL_Submit_BoundingBox(0, composed, section_idx)

C) VXL_Sort_Rasterize  (0x00754510)
   1. Bubble-sort BoxRecords by min-Z (ascending)
   2. for each BoxRecord, call VXL_Section_Rasterizer
   3. Compute screen dirty rect

D) Final blit (FUN_004AF2A0) — visibility/depth maps to PrimarySurface
```

### 5.2 The building voxel pipeline

`BuildVXLTurretMatrix @ 0x00458810` is **buildings-only**. Buildings rarely use
voxels — most YR buildings are SHP. When they do (turrets like Prism Tower's
spire), the composition reads from BuildingTypeClass:

```
M = identity
M ← M · T(+0x1748, +0x174C, +0x1750)   // VoxelBarrelOffsetToBuildingPivotPoint
M ← M · Rz(turret_yaw_subtick)         // RateTimer @ +0x388
M ← M · T(+0x173C, +0x1740, +0x1744)   // VoxelBarrelOffsetToRotatePivotPoint
M ← M · Ry(-barrel_pitch_subtick)      // RateTimer @ +0x370
M ← M · T(+0x1730, +0x1734, +0x1738)   // VoxelBarrelOffsetToPitchPivotPoint
M ← M · S_basis_only(*(double*)+0x1728) // scale basis cols only — translation column NOT scaled
```

Each "shear_col*" call right-multiplies by a translation along the current
matrix's basis axis. Each rotation right-multiplies by a basis rotation. The
sub-tick angle formula matches the unit path:

```
step  = ((rate_timer_current >> 10) + 1) >> 1  &  0x1F          // 5-bit, 0..31
angle = (step - 8) × (-PI/16)                                    // const at 0x007E4408
```

`-PI/16` per step × 32 steps = `-2π` (full rotation), confirming the 32-facing
convention used throughout the engine.

### 5.3 Drive/Ship body-matrix tilt path

The tilt path activates when `|AngleRotatedSideways| ≥ 0.005 rad` OR
`|AngleRotatedForwards| ≥ 0.005 rad` OR a slope transition is in flight
(`completion_fraction < 1.0`). Threshold const at `0x007E44E8` = double `0.005`.

The shear block re-centers the voxel after rotation:

```
combined_Z   = ftol(|cos_roll|·tilt_mag_X + |cos_pitch|·tilt_mag_Y)
partial_Y    = ftol(sin_pitch · tilt_mag_X)
remainder_Y  = ftol(tilt_mag_X − partial_Y)
partial_X    = ftol(sin_roll  · tilt_mag_Y)
remainder_X  = ftol(tilt_mag_Y − partial_X)

if pitch < 0:  remainder_Y = -remainder_Y
if roll  ≥ 0:  remainder_X = -remainder_X
```

Then:

```
A = identity
A · T_z(combined_Z)
A · T_x(remainder_Y)
A · T_y(remainder_X)

B = identity
B · Rx(roll)
B · Ry(pitch)

slope_mat = (slope == 0 ? identity : VXL_GetFacingMatrix(slope) or VXL_InterpolatedFacing(prev, curr, fraction))
fac_rot   = BuildFacingRotationMatrix(loco)

result = B × fac_rot × A × slope_mat
```

The slope-transition timer is **3 frames hard-coded** (`CDTimerClass__Start(3)`
in `DriveLocomotionClass::Process`).

### 5.4 Slope flat path (no tilt)

When `slope == 0`, `VXL_GetFacingMatrix` is **skipped** (verified at
`0x4B023E`/`0x4B0395`). The slope slot stays identity, the "other slot" stays
identity, and the result is `fac_rot × identity = fac_rot`. **No half-cell
offset is applied on flat ground.** Any half-cell offset present in the Rust
renderer for flat tiles is parity drift.

### 5.5 Aircraft body matrix

`FlyLocomotionClass::Render_Matrix` is far simpler:

```
slope = cell+0x11C
if (TechnoTypeClass+0xC95 ConsideredAircraft != 0):
    slope = 0                                  // aircraft: force flat
out = VXL_GetFacingMatrix(slope)
sub = ((RateTimer__Current() >> 10 + 1) >> 1 & 0x1F) - 8
Matrix3x4_RotateZ(out, sub × -PI/16)
```

**Aircraft body matrix has no banking, no climb/dive tilt, no take-off
rotation, no slope tilt.** All vertical orientation animation lives in the
turret/barrel state machine at `FUN_00729B40`, applied separately per-section
during the per-section composition. Missiles with `ConsideredAircraft=false`
(Dreadnought sub-launched cruise missile, V3 rocket) DO follow ground slope.

### 5.6 VXL ↔ HVA pairing rule

**Positional, by integer index.** No name lookup. The same integer
`section_idx` is used as the limb-array index in the VXL data and the
section-array index in the HVA matrix block. If the two counts differ, the
math walks off the matrix array. Westwood's pipeline ensures matching counts
at asset-build time.

### 5.7 HVA frame selection cadence

```
frame = FootClass+0x538
hva_idx = (frame % HVA->frame_count)
```

`FootClass+0x538` is the per-unit "animation frame counter". `FootClass::AI`
(0x004DA530) increments it gated on:

- `TechnoTypeClass+0x294 (WalkRate)` — divisor when moving
- `TechnoTypeClass+0x298 (IdleRate)` — divisor when idle/firing
- `g_CurrentFrameCounter % rate == 0` — the actual increment trigger
- Suppressed when piggybacked, falling, cloak-fading, or in transport

For most YR voxels, `frame_count == 1` so the modulo always yields 0 (static
unit). Animated voxels (e.g., propeller idle, falling debris) have
`frame_count > 1` and animate at `WalkRate`/`IdleRate` cadence.

`VoxelAnimClass` doesn't increment its own frame counter — its draw caller
(`VoxelAnim::Draw`, `0x0046B0C0`) passes a frame index that is typically the
global counter or 0.

---

## 6. Lighting / Normals Tables

Verified by decompiling `VXL_SimpleLighting @ 0x00758670`,
`VXL_BlinnPhongLighting @ 0x007586F0`, `VXL_MasterLighting_Init @ 0x00754CB0`,
and reading the data tables.

### 6.1 Path selection

| Caller path | Lighting function | Notes |
|---|---|---|
| `TechnoClass::Render @ 0x00706F4D` (vehicles, buildings, turrets) | `VXL_BlinnPhongLighting` (always) | Specular path |
| `VoxelAnim::Draw @ 0x0046B0E1` (projectiles, debris) | `VXL_SimpleLighting` (always) | Diffuse-only path |

**There is no flag, no INI key, no fallback that selects between them.** The
choice is determined by which class owns the voxel.

### 6.2 Normals tables

```
Dispatch table at 0x008469E0   ptrs = [NULL, 0x00846A08, 0x00846AC8, 0x00846C78, 0x00846F78]
Counts table at 0x008469F4    cnts = [0,    16,         36,         64,         245       ]
                                     mode 0  mode 1     mode 2 (TS) mode 3      mode 4 (RA2)
```

| Mode | Address | Entries | Notes |
|---|---|---|---|
| 0 | NULL | 0 | LUT untouched, only ambient triple is set |
| 1 | `0x00846A08` | 16 | Probably RA1-era — usage in stock YR retail unconfirmed |
| 2 | `0x00846AC8` | 36 | TS legacy |
| 3 | `0x00846C78` | 64 | Purpose unclear — usage in stock YR retail unconfirmed |
| 4 | `0x00846F78` | **245** (NOT 256) | RA2 standard |

For mode 4, **entries 245–249 are byte-duplicates of entry 244**, and
**entries 250–255 do not exist** in the binary. The LUT region
`0xB45990 + 245..252` (8 bytes) is never written by the lighting init.

> **Disparity.** Rust's `RA2_NORMALS` table treats indices 0..255. Iteration
> in `vxl_normals.rs` should run `0..245` for mode 4 (entries 245–249 are
> duplicates of 244 by binary truth; entries 250–255 are stale memory). If a
> voxel ever references index ≥ 250, gamemd reads stale LUT bytes and Rust
> reads `(0,0,1)` — divergent output. Frequency: rare in retail VXLs.
>
> **Disparity.** Rust comment claims "entries 252-255 are duplicates of 251".
> Wrong number range. Real binary: **entries 245-249 byte-duplicated from 244**.

> **Mode 4 first-entry sanity check (verified):** `(0.526578, -0.359621, -0.770317)`.
> Mode 4 entries 240..244: `(-0.026045, -0.397820, 0.917094)`,
> `(0.267897, -0.649041, 0.712023)`, `(0.518246, -0.284891, 0.806386)`,
> `(0.493451, -0.066533, 0.867225)`, `(-0.328188, 0.140251, 0.934143)`.

### 6.3 SimpleLighting math

```
for i in 0 .. count:
    dot = N[i].x · L.x + N[i].y · L.y + N[i].z · L.z
    if dot < 0.0:                            // double 0.0 at 0x007E2800
        g_VXL_NormalLUT[i] = 0
    else:
        g_VXL_NormalLUT[i] = ftol(dot × 16.0)  // float 16.0 at 0x007F6960
g_VXL_NormalLUT[253..255] = 0x10              // ambient triple, hardcoded
```

`Math__ftol` truncates toward zero, no upper clamp — relies on `dot × 16.0`
staying in 0..255.

### 6.4 BlinnPhongLighting math

```
diffuse  = max(0, dot(N, L))
half     = normalize(L + V)
h_dot    = max(0, dot(N, half))
specular = h_dot / (s − h_dot·s + h_dot)         // Schlick-like; s = shininess
final    = (diffuse + specular) × 16.0
g_VXL_NormalLUT[i] = ftol(max(0, final))
g_VXL_NormalLUT[253..255] = 0x10                  // ambient triple
```

**Specular strength `s = 3.0`**, passed by `TechnoClass::Render` as the 5th
argument to `VXL_Init_BlinnPhong` (constant `0x40400000` = float 3.0).

> **Disparity (parity-critical).** Rust's `blinn_phong_pages` uses
> `SPECULAR_STRENGTH = 3.4`. Binary uses `3.0`. **HIGH severity** — affects
> every voxel unit's specular highlight in every frame of every match. Schlick
> with `s=3.4` produces lower output than `s=3.0` for the same `h·N`, making
> Rust's highlights subtly weaker. Trigger frequency: every voxel unit, every
> frame.

### 6.5 Light direction

`g_VXL_LightDirection @ 0x00887470` (3 floats). Initialized once at startup by
`VXL_LightDirection_Setup @ 0x00754C00`:

1. Build identity matrix, post-multiply by Y-rotation.
2. Multiply against constant input `(-0.7071068, -0.7071068, 0.0)` (i.e., 45°
   in XY plane pointing NW in voxel local space; const `0xBF3504E6` = float
   -0.7071068).
3. Write transformed result to `g_VXL_LightDirection`.

It is **NOT called per-section per-frame**. This is a one-time global setup.
There is no INI-driven RGB diffuse/ambient — diffuse "color" = the dot-product
result mapped to a VPL page; ambient = hardcoded byte 0x10.

`_DAT_00887420 = light.x × -6.0` (`0xC0C00000` = float -6.0) is also written
here, used downstream for shadow projection.

### 6.6 Master lighting init

`VXL_MasterLighting_Init @ 0x00754CB0` runs once at game startup. Despite the
name, it does NOT set up RGB diffuse/ambient. It builds:

- 18+ pre-rotated cube view matrices for VXL bounding-box culling at
  `0x00B43F40..0x00B45498`.
- The `g_VXL_NormalVectors` array of `0x100` (256?) precomputed shadow
  projection normals at `0x00B431C0+`.
- The 32 facing matrices in `g_VXL_FacingMatrices @ 0x00B45188` (each 48
  bytes), built by `Matrix3x4_BuildFromRotateXAndFacing(facing_z, tilt_x)`
  (`0x005AE6F0`) using slope angles from `_DAT_00B43F08` (corner tilt) and
  `_DAT_00B44310` (edge tilt).

### 6.7 Constants

| Address | Type | Value | Purpose |
|---|---|---|---|
| `0x007E2800` | double | `0.0` | dot threshold (clamp negatives) |
| `0x007E1748` | float | `0.0` | specular threshold |
| `0x007F6960` | float | `16.0` | diffuse-to-page scale (`* 16` per dot) |
| `0x007F6950` | float | `-6.0` | shadow scale |
| `0x40400000` | float | `3.0` | specular strength (TechnoClass::Render arg) |
| `0x00B45990` | LUT | 256 bytes | `g_VXL_NormalLUT`; entries 253-255 hardcoded `0x10` |
| `0x00887470` | vec3 | runtime-set | `g_VXL_LightDirection` |
| `0x00887430` | vec3 | runtime-set | viewer V (Blinn-Phong half-vector input) |

---

## 7. Facing-Matrix System

Verified by decompiling `VXL_GetFacingMatrix @ 0x007559B0`,
`VXL_InterpolatedFacing @ 0x00755A40`, `Quaternion_Slerp @ 0x00646590`,
`Quaternion_ToMatrix @ 0x00646980`.

### 7.1 Facing-matrix table

`g_VXL_FacingMatrices @ 0x00B45188` — array of 48-byte matrices (`stride 0x30`),
populated by `VXL_MasterLighting_Init`. Each entry is a 3×4 row-major matrix
with `Matrix3x4_BuildFromRotateXAndFacing(facing_z, tilt_x)`, equivalent to:

```
identity → Rz(facing) → Rx(tilt) → Rz(-facing)
        = Rz(-f) ∘ Rx(t) ∘ Rz(f)
```

### 7.2 Slope-tilt mapping

| Slope idx | Compass | Z-rotation | X-tilt | Table address |
|---|---|---|---|---|
| 0 | flat | — | — | identity |
| 1 | West | 270° | edge `0xB44310` (≈29.88°) | `0xB451B8` |
| 2 | North | 180° | edge | `0xB451E8` |
| 3 | East | 90° | edge | `0xB45218` |
| 4 | South | 0° | edge | `0xB45248` |
| 5 | NW corner | 225° | corner `0xB43F08` (≈22.10°) | `0xB45278` |
| 6 | NE corner | 135° | corner | `0xB452A8` |
| 7 | SE corner | 45° | corner | `0xB452D8` |
| 8 | SW corner | 315° | corner | `0xB45308` |
| 9-12 | Mid (3-corners-up half-cell) | repeat 5-8 | corner | `0xB45338..0xB453F8` |
| 13-16 | Steep (full-cell drops) | repeat 1-4 | edge | `0xB45428..0xB45488` |
| 17-20 | Double ramps | mirror block | — | mirror at `0xB450B8..0xB45178` |

### 7.3 GetFacingMatrix (no clamp)

```
ESI = param_2 × 0x30 + 0xB45188
MOVSD.REP ES:EDI, ESI    ; copy 12 dwords (48 bytes) to *param_1
```

No bounds check; caller is responsible for `0..31` index range. `0..20` covers
all real slope types; the remaining slots are mirror copies populated at init.

### 7.4 InterpolatedFacing (genuine SLERP)

```
if (param_2 == param_3):
    copy g_VXL_FacingMatrices[param_3] directly
else:
    Quaternion_Slerp(temp_quat, &quat[param_2], &quat[param_3], fraction)
    Quaternion_ToMatrix(temp_matrix, temp_quat)
    copy temp_matrix (12 dwords) to *param_1
```

`Quaternion_Slerp @ 0x00646590` is **genuine spherical interpolation** with the
standard 3-branch logic: antipodal handling (`fVar3 + 1.0 ≤ EPS`), near-parallel
falls through to LERP coefficients `(1-t)` and `t`, general case uses
`acos(dot)` for omega and `cos(omega·(1-t))/cos(omega)`, `cos(omega·t)/cos(omega)`.

The quaternion table lives at `0x00B43188` (16 bytes per entry, 32 entries).

### 7.5 Who calls what

`VXL_GetFacingMatrix` callers: `BounceClass__Update @ 0x00439B00`,
`BulletClass__AI @ 0x004666E0`, `DriveLocomotionClass__Draw_Matrix @ 0x004AFF60`,
`FlyLocomotionClass__Render_Matrix @ 0x004CFB00`,
`LocomotionClass__Build_Shadow_Matrix @ 0x0055A7D0`,
`ShipLocomotionClass__Draw_Matrix @ 0x0069F670`,
`Turret_barrel_tilt @ 0x00729B40`, `FUN_0062BD50`, `FUN_0062C6E0`.

`VXL_InterpolatedFacing` callers: only `Drive` and `Ship`. **Aircraft and
turrets do NOT SLERP** between facings — they snap to the nearest discrete
matrix. Parity-relevant: only ground vehicles and ships interpolate facing
between two slopes during transition.

---

## 8. Voxel-Run Decoder (`dup_count` resolved)

Verified across three rasterizer variants: `FUN_007DF7C0` (standard opaque),
`VXL_Rasterizer_Mirror @ 0x00757120` (mirrored opaque), `FUN_007DF8C0` (mirrored
backward-walk variant).

The cleanest variant is `0x00757120`:

```c
remaining = size_x;                       // u8 from tailer
while (remaining != 0) {
    skip = *p;                            // u8
    pos += skip × step;                   // (precomputed step from a per-mode LUT)
    remaining -= skip;
    count = p[1];                         // u8
    p += 2;                               // past [skip, count]
    if (count != 0) {
        remaining -= count;
        do {
            color  = *p;                  // u8
            normal = p[1];                // u8 → fed to g_VXL_NormalLUT[normal]
            p += 2;
            count--;
        } while (count != 0);
    }
    p += 1;                               // SKIP trailer byte (value NOT read)
}
```

**`dup_count` (the trailer byte after each run) is consumed by every rasterizer
but its value is never read.** No comparison, no equality check, no validation.
Rust's parser must consume the byte (advance the cursor) but **must not assert
anything about its value**. Westwood's writer presumably wrote it as a copy of
`count` (or zero), but gamemd treats it as opaque padding.

The skip step is looked up from a per-rasterizer-mode table at `0x00B45590`
(populated at draw start, sample read returned all zeros at scan time —
confirming runtime initialization). Each mode of the 32-entry rasterizer table
at `g_VXL_RasterizerTable @ 0x00846840` selects a different traversal axis;
the mode is built from flag bits:

- Bit 0: from `0x008468C0[mode]` (mode-dependent base index)
- Bit 1: `g_VXL_MirrorFlag @ 0xB43180` (water reflections / shadows)
- Bit 2: `g_VXL_RenderMode @ 0xB43184` (alternate normal-encoding)
- Bit 3: `tailer.alpha == 0` (transparent flag from tailer +0xA3 — but that
  byte is `normals_mode` per §2.5; this is an alias usage)

> **Note on bit 3 alias.** The rasterizer treats tailer +0xA3 as a "transparent"
> flag (`tailer[+0xA3] == 0` → OR's bit 8 into the dispatch). This is the same
> byte `vxl_file.rs` calls `normals_mode`. The lighting path (§6) reads it as
> `normals_mode` (range 0..4). The rasterizer reads it as a flag (any nonzero
> → opaque). Both readings can coexist for normal_mode values 1..4 (all opaque)
> but mode 0 (NULL normals table) means transparent rendering. **Verify at
> implementation time that Rust treats the byte consistently.**

### 8.1 Voxel-to-screen projection

Per voxel: `iVar13` is a packed 16.16 fixed-point screen position. Per-mode
deltas:

- Per-skip: `iVar13 += skip × (transform_x_increment_lookup)`
- Per-voxel: `iVar13 += transform_x_increment`
- Per-column: `iVar13 += transform_y_increment`
- Per-row: `iVar13 += transform_z_increment`

Pixel coordinates extracted as `((iVar13 >> 24) & 0xFF, (iVar13 >> 8) & 0xFF)`.

The 256×256 visibility/depth maps live at `g_VXL_VisibilityMap @ 0xB2FF78` and
`g_VXL_DepthMap @ 0xB1D5E0`; model origin = `(128, 128)`. Each rasterized voxel
writes one byte to each. After rasterization, the dirty rect is computed for
partial-clear next frame.

The fixed-point multiplier is `0x007E2224 = 65536.0f` (16.16 fixed-point).

### 8.2 Rasterizer edge cases

| Case | Behavior |
|---|---|
| `tailer.size_x == 0` | `do…while(uVar9 != 0)` loop wraps to `0xFFFFFFFF` → 4 GB walk. **Unguarded UB.** Real VXLs never have size_x = 0. |
| `tailer.size_y == 0` | Same UB. |
| Span with `count == 0` | Inner emit loop skipped. Trailer byte still consumed. Outer loop continues. |
| `span_start[col] == -1` | Span skipped via `if (-1 < iVar2)` gate. **Canonical empty-column encoding.** |
| Voxel `color_index == 0` | Written unconditionally. Color-0 transparency handled at blit stage by the palette-translate Blitter masking palette index 0. |
| `tailer.alpha == 0` | OR's bit 3 → transparent rasterizer variant. |
| `g_VXL_MirrorFlag != 0` | OR's bit 1 → mirror-mode (cells walked in reverse X order). |

---

## 9. Disparity List vs Current Rust Parsers

Consolidated across all sections; severity = player-visibility × frequency.

### 9.1 HIGH severity (immediate fix required)

| # | Location | Disparity | Impact |
|---|---|---|---|
| 1 | `vxl_file.rs:VXL_HEADER_SIZE = 802` | Real header is **32 bytes** + variable palette (`palette_count × 770`) | Wrong for any VXL with `palette_count != 1`. Need to parse 32-byte header, then seek `palette_count × 770` bytes, then proceed. |
| 2 | `vxl_normals.rs:SPECULAR_STRENGTH = 3.4` | Binary uses **3.0** (TechnoClass::Render passes `0x40400000`) | Every voxel unit, every frame. Highlights subtly wrong. |

### 9.2 MEDIUM severity

| # | Location | Disparity | Impact |
|---|---|---|---|
| 3 | `vxl_file.rs` tailer `+0x0C` labeled "limb_identifier u32" | Real field is **`f32 scale`** | If consumed, wrong type will produce nonsense values. If unused, cosmetic. |
| 4 | `vxl_normals.rs` RA2 normals iterates 0..256 | Mode 4 has **245 entries**; entries 245–249 are dupes of 244; 250–255 don't exist | Rust's tail (250–255 = (0,0,1) padding) diverges from gamemd's stale memory. Rare in retail. |
| 5 | `vxl_normals.rs` comment "entries 252-255 are duplicates of 251" | Real is **245-249 byte-duplicated from 244** | Comment misleads; fix comment + iteration count together. |

> **2026-07-19 correction to rows 4–5** (verified live — `read_memory
> 0x00847AE8` / `0x00847AF4`): the binary's mode-4 table ends at exactly 245
> entries; the bytes at entries 245+ are unrelated RTTI data, **not**
> byte-duplicates of entry 244 (the "dupes" phrasing above described the
> community dump, not the binary). "Rare in retail" is superseded: the
> retail_goldens corpus scan proved index 255 occurs in 8 retail files (1,931
> voxels, incl. sreftur.vxl), and the shade gamemd gives it is a deliberate
> constant — VPL page 0x10 via the LUT ambient tail. Full chain:
> `VXL_STALE_NORMAL_255_AMBIENT_PAGE_GHIDRA_REPORT.md`.
| 6 | Mode 1 (16 entries) and Mode 3 (64 entries) tables | Implemented in binary, missing in Rust | Stock YR retail usage of modes 1/3 is unconfirmed; if any retail VXL uses them, Rust falls back to default and lights wrongly. |

### 9.3 LOW severity

| # | Location | Disparity | Impact |
|---|---|---|---|
| 7 | `hva_file.rs` reads and stores section names | Binary **seeks past** them | Memory cost only; matrix offsets land at the same place. Keep for diagnostics. |
| 8 | `hva_file.rs` rejects `section_count == 0` | Binary accepts (`operator_new(0)`) | No real HVA has zero sections. Defensive rejection is OK. |
| 9 | `vpl_file.rs:get_palette_index` clamps page index | Binary does **not** clamp | Stock `voxels.vpl` has enough pages; clamp is harmless defense. |
| 10 | `vxl_file.rs` magic enforcement | Binary doesn't validate | Rust's enforcement is fine. |
| 11 | VXL `dup_count` byte | Rust comments "validation/padding — unused" | **VERIFIED CORRECT**. Do not assert any equality. |
| 12 | `vxl_normals.rs` `get_normal()` returns `Vec3::Z` for unknown modes | Mode 0 = NULL table, LUT untouched | Different fallback behavior. Not parity-critical for retail content. |

### 9.4 Verified-correct (no disparity)

- `SECTION_HEADER_SIZE = 28` ✓
- `SECTION_TAILER_SIZE = 92` ✓
- `HVA_MIN_SIZE = 24` ✓
- `SECTION_NAME_SIZE = 16` ✓
- HVA frame_count at file offset 0x10 ✓
- HVA section_count at file offset 0x14 ✓
- Matrix layout: 3×4 row-major, 12 f32, translation at indices `[3,7,11]` ✓
- HVA matrix index = `frame × section_count + section` ✓
- VXL column iteration `y × size_x + x` ✓
- VXL `span_start[col] == -1` empty-column sentinel ✓
- VXL run encoding `[skip][count][(c,n)×count][trailer]` ✓
- Per-mode VPL output formula `pages[(lut_byte << 8) | color_byte]` ✓

---

## 10. TS-Legacy Risk Register

Per CLAUDE.md, every flag/branch on this list is annotated **Active in YR:
Yes/No/Conditional**.

| # | Item | Active in YR | Evidence |
|---|---|---|---|
| 1 | Full palette read + remap path (`iStack_4 != 0` in VXL_Load_File) | **No** | All callers pass literal `0`. Branch + `VXL_BuildRemapTable` + `VXL_NearestColorMatch` are dead code. |
| 2 | `VXL_BuildRemapTable @ 0x00758B70` | **Conditional** | Reachable only via dead VXL palette branch and `FUN_00753b70` (light-table init). The latter does run at game init. |
| 3 | `VXL_NearestColorMatch @ 0x00758EA0` | **No** | Only reachable from `VXL_BuildRemapTable`. Effectively dead. |
| 4 | `VXL_BlinnPhongLighting @ 0x007586F0` | **Yes** | Always invoked for vehicles/buildings/turrets. NOT TS-legacy. |
| 5 | `VXL_SimpleLighting @ 0x00758670` | **Yes** | Always invoked for VoxelAnims. NOT TS-legacy. |
| 6 | Two `VXL_Sort_Rasterize` entries (`0x007542F0` and `0x00754510`) | **Both Yes** | `0x754510` is the primary real-time path; `0x7542F0` is the cache-pipeline integer-rect variant. NOT a TS duplicate. |
| 7 | `VXL_Rasterizer_Mirror @ 0x00757120` | **Conditional** | Triggered when `g_VXL_MirrorFlag != 0`. Used for water-reflection and shadow paths. Live in YR for any map with water. |
| 8 | `VXL_Submit_BoundingBox @ 0x007540F0` | **Yes** | Per-section dispatch entry for every voxel draw. NOT TS-legacy. (The dev-only smell was wrong.) |
| 9 | Normals mode 0 (NULL table) | **Conditional** | Implemented in dispatch. Asset usage in retail YR unconfirmed. |
| 10 | Normals mode 1 (16 entries, RA1-era) | **Conditional** | Implemented. Asset usage unconfirmed. |
| 11 | Normals mode 2 (TS, 36 entries) | **Conditional** | Implemented. Some legacy TS-era VXLs may still ship in YR retail. Worth a sweep. |
| 12 | Normals mode 3 (64 entries) | **Conditional** | Implemented. Asset usage unconfirmed. |
| 13 | Slope types 17–20 (double ramps) | **Conditional** | Table populated, but cell slope bytes ≥ 9 may not appear in any YR retail tile. Defer until encountered. |
| 14 | Mirror matrices `0xB450B8..0xB45178` (negative slope offsets) | **Unknown** | Populated by `VXL_MasterLighting_Init` but no observed caller indexes negatively into `g_VXL_FacingMatrices`. May be TS leftover. |
| 15 | `RockingUpdate` spring-damper (`techno+0x328/0x32C` AngleRotatedSideways/Forwards) | **Yes** | Used by Drive/Ship Draw_Matrix tilt path for cosmetic shake on weapon fire / hard turns. |
| 16 | `ConsideredAircraft = false` slope-tilt path for Fly locomotor | **Yes (rare)** | Triggered for missile units (Dreadnought sub-launched cruise missile, V3 rocket) using FlyLocomotion. Visually irrelevant due to small size + speed but live. |
| 17 | `g_VXL_RenderMode @ 0xB43184` (rasterizer flag bit 2) | **Conditional** | Set somewhere (callers not traced); selects alternate normal-stride rasterizers at index +2 from base. May be a special draw mode. |
| 18 | `tailer.alpha == 0` rasterizer entries 12–15 | **Yes** | Triggered when `normals_mode == 0` (NULL normals → transparent rendering). Dispatches to alpha-blend rasterizers. |
| 19 | `g_VXL_FacingMatrices` slot 0 explicit setup | **Yes** | Identity initialized at startup, used for flat-ground draws. NOT TS-legacy. |

**Summary:** the only outright dead VXL pipeline code is the palette
full-read branch (#1) and `VXL_NearestColorMatch` (#3). Everything else is at
least conditionally reachable in YR. The Rust implementation should:

- Skip palette loading entirely (just seek past).
- Implement all 5 normals modes (or fall back gracefully) since 1/2/3 are
  reachable for legacy assets.
- Keep both rasterizer-sort variants implemented (or unify them with no
  observable difference).

---

## 11. Voxel Shadow Pipeline

Verified by `ObjectClass::DrawVoxelShadow @ 0x005F5B90`,
`DriveLocomotionClass::Shadow_Matrix @ 0x004B0410`,
`LocomotionClass::Build_Shadow_Matrix @ 0x0055A7D0`.

```
ObjectClass::DrawVoxelShadow:
    locomotion = this.GetLocomotion()
    shadow_mat = locomotion.vtable[Shadow_Matrix](this)
    -> rasterize VXL through normal section pipeline, but with
       g_VXL_NormalLUT untouched (no VXL_Init_BlinnPhong call) and
       a different blitter that color-keys to the shadow color
```

Drive/Ship `Shadow_Matrix` builds:

```
M = VXL_GetFacingMatrix(facing)
M ← M · Rz((quantized_timer - 8) × angle_per_step)    // small wobble
```

Aircraft (`FlyLocomotion`) overrides via vtable to a different matrix (clamped
to ground level for shadow).

The shadow rasterization uses the **same rasterizer table** as the body, but
with `g_VXL_RenderMode = 0` set, dispatching to `FUN_007DF7C0` /
`FUN_007DF8C0` (or mirror variants). The final blit applies a "shadow"
blitter that alpha-darkens or palette-translates to a darken color, producing
the silhouette.

Shadows do NOT use the precomputed `g_VXL_NormalVectors @ 0xB431C0` — that
array is for diffuse lighting normals, not shadow normals.

---

## 12. Function Inventory Resolution

Every function from the plan's Section 3 inventory and where it ended up in
this report:

| # | Phase | Address | Name | Resolved |
|---|---|---|---|---|
| 1 | 1 | `0x00755DB0` | `VXL_Load_File` | §2.1–2.5 — full byte layout |
| 2 | 1 | `0x005BD5C0` | `HVA_Load_File` (located!) | §3 — full byte layout |
| 3 | 1 | `0x00758950` | palette workspace alloc helper | §2.2 — dead in YR |
| 4 | 1 | `0x00758B70` | `VXL_BuildRemapTable` | §10 #2 — dead in YR (cond.) |
| 5 | 1 | `0x007559B0` | `VXL_GetFacingMatrix` | §7.3 |
| 6 | 1 | `0x00755A40` | `VXL_InterpolatedFacing` | §7.4 |
| 7 | 1 | `0x00458810` | `BuildVXLTurretMatrix` | §5.2 — buildings only |
| 8 | 1 | `0x00453C98` | `GetTurretDrawPosition` | §5.2 caller |
| 9 | 1 | `0x004AFF60` | `DriveLocomotionClass__Draw_Matrix` | §5.3 |
| 10 | 1 | `0x0069F670` | `ShipLocomotionClass__Draw_Matrix` | §5.3 — byte-identical to Drive |
| 11 | 1 | `0x004CFB00` | `FlyLocomotionClass__Render_Matrix` | §5.5 |
| 12 | 1 | `0x00756590` | `VXL_Section_Rasterizer` | §8 — voxel-run decode |
| 13 | 1 | `0x00754CB0` | `VXL_MasterLighting_Init` | §6.6 |
| 14 | 1 | `0x00758670` | `VXL_SimpleLighting` | §6.3 |
| 15 | 2 | `0x00758EA0` | `VXL_NearestColorMatch` | §10 #3 — effectively dead |
| 16 | 2 | `0x00756860` | `VXL_Quad_Rasterizer` | §8 — quad-emit inner loop |
| 17 | 2 | `0x007542F0` | `VXL_Sort_Rasterize` (variant A) | §10 #6 — both live |
| 18 | 2 | `0x00754510` | `VXL_Sort_Rasterize` (variant B) | §10 #6 — primary real-time |
| 19 | 2 | `0x00753D00` | `VXL_Init_BlinnPhong` | §6 — wrapper for #20 |
| 20 | 2 | `0x007586F0` | `VXL_BlinnPhongLighting` | §6.4 |
| 21 | 2 | `0x007DF9C0` | `VXL_Rasterizer_RenderMode` | §4.3 — VPL output formula |
| 22 | 2 | `0x00753E00` | `VXL_Clear_TileMap` | §5.1 step B.4 |
| 23 | 2 | `0x00753F90` | `VXL_Submit_Billboard` | rasterizer pipeline plumbing |
| 24 | 2 | `0x005AE750` | `Matrix3x4_BuildAxisAngleRotation` | helper |
| 25 | 2 | `0x005AF980` | `Locomotion_Matrix` | matrix multiplier (out = A × B) |
| 26 | 2 | `0x00754C00` | `VXL_LightDirection_Setup` | §6.5 |
| 27 | 2 | `0x00749F30` | `VoxelAnimClass__AI` | §5.7 — does NOT increment frame |
| 28 | 3 | `0x00757120` | `VXL_Rasterizer_Mirror` | §10 #7 — water/shadow path |
| 29 | 3 | `0x007540F0` | `VXL_Submit_BoundingBox` | §10 #8 — per-section dispatch entry |
| 30 | 3 | `0x005F5B90` | `ObjectClass__DrawVoxelShadow` | §11 |
| 31 | 3 | `0x0046B0C0` | `VoxelAnim__Draw` | §5.7 — passes frame to draw call |
| 32 | 3 | `0x00748AF0` | `VoxelDebris__Render` | downstream consumer of #14 |
| 33 | 3 | `0x0043E63E` | `BuildingClass turret matrix path` | §5.2 — buildings rare in YR |
| 34 | 3 | `0x007493B0` | `VoxelAnimClass__Constructor` | spawn init |
| 35 | 3 | `0x0074B050` | `VoxelAnimTypeClass__ReadINI` | INI parser (caller of VXL load) |

---

## 13. Open Questions (deferred)

Resolved during execution but worth tracking for future work:

1. **Mode 1 / Mode 3 normals tables** — implemented in binary, but stock YR
   retail asset usage unconfirmed. To resolve: enumerate the `normals_mode`
   byte at `tailer +0xA3` for every retail `.vxl` file. Cost: a one-shot
   scanner against retail assets.

2. **Slope types 9–20 in YR retail tiles** — facing-matrix table populated but
   cell `slope_byte ≥ 9` may not be produced by any retail tile. To resolve:
   enumerate retail TMP/TMA tiles for cells with high slope bytes.

3. **Mirror matrices at `0xB450B8..0xB45178`** — populated by master init but
   no observed caller indexes negatively into `g_VXL_FacingMatrices`. May be
   TS leftover for upside-down/inverted-normal slopes. **Not parity-critical**
   unless a retail YR tile reaches them.

4. **VPL header bytes 0–7 and 12–15 semantics** — known to be in the file
   but no consumer was traced reading them. Likely informational
   (first/last remap range, version flag). **Not parity-critical** for
   stock content.

5. **`g_VXL_RenderMode @ 0xB43184` setter** — selects alternate normal-stride
   rasterizers. Setter not traced. **Defer until a render bug surfaces that
   hints at this code path.**

6. **`techno+0x328/0x32C` (`AngleRotatedSideways`/`Forwards`) source** — fed
   by `RockingUpdate` (`0x70B570`). Whether YR uses it for cosmetic shake on
   weapon fire vs simulated braking deceleration is not traced. **Not
   matrix-construction-critical.**

7. **Tailer +0xA3 byte alias (`normals_mode` vs `alpha`)** — the lighting path
   reads it as `normals_mode` (range 0..4). The rasterizer reads it as a
   "transparent" flag (`== 0` → alpha). Both readings overlap at mode 0 (which
   means "no lighting, transparent rendering"). **Verify at implementation
   time that Rust treats the byte consistently across both code paths.**

8. **`FUN_005ae5e0` purpose** — confirmed identity 48-byte memcpy. May be a
   leftover from when it WAS a transposition (TS era?), or a defensive
   placeholder. In current YR it's a no-op. **Not parity-critical.**

9. **Building voxel turret coverage** — voxel-bodied buildings are rare in
   stock YR (most use SHP). The composition path at `BuildVXLTurretMatrix`
   exists and is reachable, but the asset count is small. **Implement when
   the first voxel-bodied building appears in scope.**

---

## 14. Sources

**Ghidra addresses decompiled or disassembled** (>50 functions):

`0x00755DB0` (VXL_Load_File), `0x005BD5C0` (HVA_Load_File), `0x005BD570`
(HVA wrapper), `0x005BD5A0` (HVA destructor), `0x005AE5E0` (matrix memcpy),
`0x00758950` (palette workspace), `0x00758B70` (VXL_BuildRemapTable),
`0x00758EA0` (VXL_NearestColorMatch), `0x007589C0` (palette destructor),
`0x007564B0` (tailer accessor), `0x00753B70` (VPL load wrapper),
`0x00758A30` (VPL file read), `0x00753C70` (VXL state accessor),
`0x007559B0` (VXL_GetFacingMatrix), `0x00755A40` (VXL_InterpolatedFacing),
`0x00646590` (Quaternion_Slerp), `0x00646980` (Quaternion_ToMatrix),
`0x00753D00` (VXL_Init_BlinnPhong), `0x00753C80` (VXL_Init_Simple),
`0x00753C90` (VXL_Init_Simple variant), `0x00754CB0` (VXL_MasterLighting_Init),
`0x00754C00` (VXL_LightDirection_Setup), `0x00754A20` (VXL_Init_CornerTiltAngle),
`0x00754A50` (VXL_Init_EdgeTiltAngle), `0x00754910..0x007549E0` (camera
init helpers), `0x00758670` (VXL_SimpleLighting), `0x007586F0`
(VXL_BlinnPhongLighting), `0x00756590` (VXL_Section_Rasterizer),
`0x00756860` (VXL_Quad_Rasterizer), `0x007DF7C0` (standard opaque rasterizer),
`0x007DF8C0` (mirror opaque rasterizer), `0x007DF9C0` (lit rasterizer),
`0x00757120` (VXL_Rasterizer_Mirror), `0x007542F0` & `0x00754510`
(VXL_Sort_Rasterize variants), `0x00754220` (VXL_Submit_BoundingBox),
`0x00753F90` (VXL_Submit_Billboard), `0x00753E00` (VXL_Clear_TileMap),
`0x00458810` (BuildVXLTurretMatrix), `0x00453C98` (GetTurretDrawPosition),
`0x0043DA80` (BuildingClass::Draw_VXL_Body), `0x004AFF60` (Drive Draw_Matrix),
`0x004B0540` (Drive::Process), `0x004CFB00` (Fly Render_Matrix),
`0x0069F670` (Ship Draw_Matrix), `0x00729B40` (Turret/barrel tilt SM),
`0x0055A730` (BuildFacingRotationMatrix), `0x005AE6F0`
(Matrix3x4_BuildFromRotateXAndFacing), `0x005AE750`
(Matrix3x4_BuildAxisAngleRotation), `0x005AE980/9B0/9E0` (matrix shears),
`0x005AEAD0/AF0/B20` (matrix scales), `0x005AEF60/F080/F1A0` (matrix rotations),
`0x005AF980` (Locomotion_Matrix), `0x00565730` (CellClass::Get_Cell_At),
`0x004C93D0` (RateTimer::Current), `0x004B4D70` (CDTimerClass::Remaining),
`0x00706ED0` (TechnoClass::Render), `0x00706640` (TechnoClass::Draw),
`0x004144B0` (AircraftClass::Draw_It), `0x00744470` (UnitClass::Draw_It),
`0x0043D290` (BuildingClass_DrawBody), `0x004E0240` (BuildingClass::Draw),
`0x005F5B90` (ObjectClass::DrawVoxelShadow), `0x004B0410` (Drive Shadow_Matrix),
`0x0055A7D0` (LocomotionClass::Build_Shadow_Matrix), `0x00749F30`
(VoxelAnimClass::AI), `0x007493B0` (VoxelAnimClass constructor),
`0x0074AD80` (VoxelAnimTypeClass constructor), `0x0074B050`
(VoxelAnimTypeClass::ReadINI), `0x0046B0C0` (VoxelAnim::Draw),
`0x00748AF0` (VoxelDebris::Render), `0x004DA530` (FootClass::AI),
`0x007C5F00` (Math__ftol).

**Memory reads:**

`0x008469E0..0x008469F8` (mode dispatch tables), `0x00846A08+` (mode 1
normals), `0x00846AC8+` (mode 2 TS normals), `0x00846C78+` (mode 3 normals),
`0x00846F78..0x00847A88+` (mode 4 RA2 normals + tail), `0x008468C0..0x008468FF`
(rasterizer orientation table), `0x00846840..0x008468BF`
(g_VXL_RasterizerTable), `0x007E1718` (1.0), `0x007E44E8` (0.005),
`0x007E4408` (-π/16), `0x007E2AC8` (1.0), `0x007E1748` (0.0), `0x007E2800`
(0.0), `0x007F6960` (16.0), `0x007F6950` (-6.0), `0x40400000` (3.0),
`0x00B44310` (edge tilt rad), `0x00B43F08` (corner tilt rad), `0x00B43F00`
(π/3), `0x00B43ED8` (π/2), `0x00B45578` (LevelHeight = 104), `0x00B45990`
(g_VXL_NormalLUT base), `0x00887470` (g_VXL_LightDirection),
`0x00887430` (viewer V), `0x00B45188+` (g_VXL_FacingMatrices entries 0..16),
`0x00B43188` (quaternion table base), `0x00826800+` (string constants for
asset filenames).

**Documents referenced (existing research):**
`VOXEL_RENDERING_ANALYSIS.md`, `VOXEL_SLOPE_TILT_SYSTEM.md`,
`VXL_DRAW_MATRIX_GHIDRA_REPORT.md`, `VOXELANIMCLASS_GHIDRA_REPORT.md`,
`OBJECTCLASS_DRAW_LIMBO_CELLLIST.md`, `TACTICAL_RENDER_PIPELINE_GHIDRA_REPORT.md`,
`UNIT_DRAW_EXTRAS_REPORT.md`, `RENDERING_PARITY_CHECKLIST.md`,
`BUILDING_SYSTEMS_GHIDRA_REPORT.md`,
`AIRCRAFTTYPECLASS_COMPLETE_GHIDRA_REPORT.md`,
`SHIP_VS_DRIVE_LOCOMOTION_COMPARISON.md`.

**INI files checked:** `ini/rulesmd.ini`, `ini/artmd.ini`.

**Rust source surveyed:**
`src/assets/vxl_file.rs`, `src/assets/hva_file.rs`, `src/assets/vxl_decode.rs`,
`src/assets/vpl_file.rs`, `src/render/vxl_normals.rs`,
`src/render/vxl_raster.rs`, `src/render/vxl_compute.rs`,
`src/render/unit_atlas.rs`, `src/bin/audit-assets.rs`.

**Investigation plan executed:**
`docs/plans/2026-05-10-vxl-hva-file-format-investigation-plan.md`.
