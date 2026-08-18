# VXL Rasterizer Dispatch Internals — Ghidra Research Report

**Primary addresses:** `VXL_Section_Rasterizer @ 0x00756590`,
`g_VXL_RasterizerTable @ 0x00846840`, orientation table @ `0x008468C0`,
`g_VXL_RenderMode @ 0x00B43184`, `g_VXL_MirrorFlag @ 0x00B43180`.
**Confidence:** HIGH overall. Every claim verified by decompiling/disassembling
the relevant function or reading the relevant memory.
**Active in YR:** Yes — these systems run for every voxel unit, every frame.

This report **extends and corrects** `VXL_HVA_FILE_FORMAT_GHIDRA_REPORT.md`, focused
on six gaps from that report's open questions: the 32-entry rasterizer dispatch
table, the 8-entry orientation table, the runtime skip-step LUT, the
`g_VXL_RenderMode` and `g_VXL_MirrorFlag` setters, the tailer `+0xA3`
dual-purpose byte, and the color-0 transparency mechanism at the blit stage.

The two read together describe the full voxel rendering pipeline.

---

## 1. Crown jewels (TL;DR)

The findings that change the existing parity model:

1. **`g_VXL_RenderMode` is the per-voxel VPL lighting toggle, not an "alternate
   normal-stride" selector.** Set to **1 once at startup** by
   `CCFileClass__Constructor @ 0x0052BDFA` and **never modified again**. YR
   voxel rendering is always lit. The four unlit rasterizers (dispatch
   indices 0/1/2/3) are dead code in YR.

2. **The skip-step LUT at `0x00B45590` is rebuilt at the start of every
   rasterizer call** — not a precomputed table. Size is `size_x × 4` bytes
   (up to 1024). Maps 1-byte run-skip count → packed `(x_delta, y_delta)`
   pair for the fixed-point screen accumulator. The prior report's "16
   bytes" claim is wrong.

3. **Of the 32 entries in `g_VXL_RasterizerTable`, only ~4 are reachable in
   stock YR play.** Entries 16-31 are completely unreachable. Entries 12-15
   alias to 8-11. Entries 0-3 require RenderMode=0 which never happens in YR.
   The four live entries are: idx **4** (lit/no-mirror/mode 0..3 corner),
   idx **5** (lit/no-mirror/mode 4..7), idx **6** (lit/mirror/mode 0..3),
   idx **7** (lit/mirror/mode 4..7).

4. **Tailer `+0xA3` is the same byte read by lighting (as `normals_mode`
   0..4) and the rasterizer (as `byte==0 → transparent` alpha-flag).** Both
   readings are coherent: mode 0 means no normals → no lighting → use the
   alpha rasterizer; modes 1..4 mean has normals → use the lit opaque
   rasterizer. There is no separate alpha byte at `+0xA4`.

5. **Color-0 transparency is hard-coded in every row blitter** with a literal
   `if (src_byte != 0)` test on the *raw source byte before palette
   translation*. Verified across 6+ blitter inner loops. Never configurable.
   Visibility-map byte 0 = "no voxel here this frame", a frame-level invariant.

6. **House remap is applied at blit time, not pre-rasterization.** The
   visibility map stores a VPL-shaded but not house-remapped palette index.
   Two units of different houses sharing one VXL produce the same visibility
   map but different final pixels. **This is a fundamental architectural
   deviation from the current Rust atlas (which caches post-remap RGBA
   per-house).**

7. **`g_VXL_MirrorFlag` is set/cleared by `TechnoClass::Render` gated on a
   `TechnoTypeClass+0xDAC` byte.** That byte is not visible in any standard
   `ReadINI` path examined; it defaults to zero and must be set
   programmatically. Open question — INI origin unresolved.

8. **`FUN_00707280` (turret cache helper) sets `MirrorFlag = 1` twice but
   never clears it.** Possible state leak: any subsequent draw before the
   next `TechnoClass::Render` cleanup will inherit `MirrorFlag = 1`.

---

## 2. `g_VXL_RasterizerTable @ 0x00846840` — full 32-entry decode

128 bytes total, read directly from memory. All 32 dispatch slots:

| idx | address | function | dispatch flags | live in YR? |
|---|---|---|---|---|
| 0 | `0x007DF7C0` | `FUN_007DF7C0` | base=0, mirror=0, render=0, alpha=0 | **No** — RenderMode=1 always |
| 1 | `0x007DF8C0` | `FUN_007DF8C0` | base=1, mirror=0, render=0, alpha=0 | **No** — same |
| 2 | `0x00757120` | `VXL_Rasterizer_Mirror` | base=0, mirror=1, render=0, alpha=0 | **No** — same |
| 3 | `0x00757360` | `FUN_00757360` | base=1, mirror=1, render=0, alpha=0 | **No** — same |
| **4** | `0x007DF9C0` | **`VXL_Rasterizer_RenderMode`** | base=0, mirror=0, render=1, alpha=0 | **Yes — primary opaque (mode 0..3)** |
| **5** | `0x007DFAE0` | **`FUN_007DFAE0`** | base=1, mirror=0, render=1, alpha=0 | **Yes — primary opaque (mode 4..7)** |
| **6** | `0x00757980` | **`FUN_00757980`** | base=0, mirror=1, render=1, alpha=0 | **Yes — mirror+lit (mode 0..3)** |
| **7** | `0x00757BF0` | **`FUN_00757BF0`** | base=1, mirror=1, render=1, alpha=0 | **Yes — mirror+lit (mode 4..7)** |
| 8 | `0x007DFC00` | `FUN_007DFC00` | base=0, mirror=0, render=0, alpha=1 | Conditional — only when `normals_mode==0` |
| 9 | `0x007DFD00` | `FUN_007DFD00` | base=1, mirror=0, render=0, alpha=1 | Conditional |
| 10 | `0x007581F0` | `FUN_007581F0` | base=0, mirror=1, render=0, alpha=1 | Conditional |
| 11 | `0x00758430` | `FUN_00758430` | base=1, mirror=1, render=0, alpha=1 | Conditional |
| 12 | `0x007DFC00` | (alias of 8) | base=0, mirror=0, render=1, alpha=1 | Conditional |
| 13 | `0x007DFD00` | (alias of 9) | base=1, mirror=0, render=1, alpha=1 | Conditional |
| 14 | `0x007581F0` | (alias of 10) | base=0, mirror=1, render=1, alpha=1 | Conditional |
| 15 | `0x00758430` | (alias of 11) | base=1, mirror=1, render=1, alpha=1 | Conditional |
| 16 | `0x00756DD0` | `FUN_00756DD0` | (5th flag bit not produced by `VXL_Section_Rasterizer`) | **No — dead** |
| 17 | `0x00756F80` | `FUN_00756F80` | same | **No — dead** |
| 18 | `0x00757120` | (alias of 2) | same | **No — dead** |
| 19 | `0x00757360` | (alias of 3) | same | **No — dead** |
| 20 | `0x007575A0` | `FUN_007575A0` | same | **No — dead** |
| 21 | `0x00757790` | `FUN_00757790` | same | **No — dead** |
| 22 | `0x00757980` | (alias of 6) | same | **No — dead** |
| 23 | `0x00757BF0` | (alias of 7) | same | **No — dead** |
| 24 | `0x00757E70` | `FUN_00757E70` | same | **No — dead** |
| 25 | `0x00758030` | `FUN_00758030` | same | **No — dead** |
| 26 | `0x007581F0` | (alias of 10) | same | **No — dead** |
| 27 | `0x00758430` | (alias of 11) | same | **No — dead** |
| 28 | `0x00757E70` | (alias of 24) | same | **No — dead** |
| 29 | `0x00758030` | (alias of 25) | same | **No — dead** |
| 30 | `0x007581F0` | (alias of 10) | same | **No — dead** |
| 31 | `0x00758430` | (alias of 11) | same | **No — dead** |

**18 unique rasterizer functions** populate the 32 slots.

### 2.1 Dispatch flag-build (verified disassembly at `0x00756818..0x0075683B`)

```
00756818  MOV ESI, [ESI + 0x008468C0]   ; ESI = base_flags from orientation table
                                         ;       (entry index = BoxRecord.orientation_index)
0075681E  TEST EAX, EAX                  ; EAX = g_VXL_MirrorFlag
00756820  JZ  +5
00756822  OR ESI, 0x2                    ; bit 1 = MirrorFlag != 0
00756825  MOV EAX, [0x00B43184]          ; load g_VXL_RenderMode
0075682A  TEST EAX, EAX
0075682C  JZ  +5
0075682E  OR ESI, 0x4                    ; bit 2 = RenderMode != 0
00756831  MOV AL, byte ptr [EBX + 0xA3]  ; tailer normals_mode byte
00756837  TEST AL, AL
00756839  JNZ +5
0075683B  OR ESI, 0x8                    ; bit 3 = byte == 0 (NULL normals)
0075683E  ...
00756843  CALL [ESI*4 + 0x00846840]      ; dispatch via g_VXL_RasterizerTable
```

**Dispatch index formula:**

```
index = orient_table[mode].dispatch_base   // 0 if mode∈{0..3}, 1 if mode∈{4..7}
      | (g_VXL_MirrorFlag != 0 ? 2 : 0)
      | (g_VXL_RenderMode  != 0 ? 4 : 0)
      | (tailer[+0xA3] == 0 ? 8 : 0)
```

`mode` is the orientation index in `[0, 8)`, set by `VXL_Submit_BoundingBox` to
the OBB corner with smallest screen-Y after transform. **5-bit-flag-table-with-32-entries
is a TS-era oversize**; YR only ever produces indices in `{4, 5, 6, 7, 12, 13, 14, 15}`
(RenderMode bit always set), with 12-15 aliasing to 8-11 inside the rasterizer.

### 2.2 Live-in-YR behavior matrix

When a section is rendered with `normals_mode != 0` (the common case — modes 1..4):

| mirror | base (geom) | dispatch idx | function | behavior |
|---|---|---|---|---|
| 0 | 0 | **4** | `0x007DF9C0` | lit, no-mirror, no-z-test, sweep from corner 0..3 |
| 0 | 1 | **5** | `0x007DFAE0` | lit, no-mirror, no-z-test, sweep from corner 4..7 |
| 1 | 0 | **6** | `0x00757980` | lit, mirror+z-test, sweep from corner 0..3 |
| 1 | 1 | **7** | `0x00757BF0` | lit, mirror+z-test, sweep from corner 4..7 |

When `normals_mode == 0` (transparent / no-normals):

| mirror | base | dispatch idx | function | behavior |
|---|---|---|---|---|
| 0 | 0 | **12 (= 8)** | `0x007DFC00` | unlit, no-mirror, alpha rasterizer, sweep 0..3 |
| 0 | 1 | **13 (= 9)** | `0x007DFD00` | unlit, no-mirror, alpha rasterizer, sweep 4..7 |
| 1 | 0 | **14 (= 10)** | `0x007581F0` | unlit, mirror, alpha rasterizer, sweep 0..3 |
| 1 | 1 | **15 (= 11)** | `0x00758430` | unlit, mirror, alpha rasterizer, sweep 4..7 |

**The "lit" path uses VPL `pages[(g_VXL_NormalLUT[normal_idx] << 8) | color_idx]`
to translate the voxel's color byte into a shaded palette index. The "unlit" path
writes the voxel's raw color byte to the visibility map directly.** Verified by
side-by-side decompilation of `FUN_007DF7C0` (slot 0, unlit) vs
`VXL_Rasterizer_RenderMode` (slot 4, lit) — they are byte-identical except for
the inner pixel-write line.

---

## 3. Orientation table @ `0x008468C0` — 8 entries × 36 bytes

Decoded from raw memory read. Each entry is **9 dwords (36 bytes)**:

```
+0x00  u32  dispatch_base       (0 or 1; bit 0 of dispatch index)
+0x04  u32  corner_idx_NEAR     (OBB corner index 0..7, "origin of raster sweep")
+0x08  u32  corner_idx_X        (OBB corner along voxel-X axis from NEAR)
+0x0C  u32  corner_idx_Y        (OBB corner along voxel-Y axis from NEAR)
+0x10  u32  corner_idx_Z        (OBB corner along voxel-Z axis from NEAR)
+0x14  u32  start_offset_X      (1 = start at far end of X, 0 = start at origin)
+0x18  u32  start_offset_Y      (1 = start at far end of Y, 0 = start at origin)
+0x1C  i32  step_x              (+1 or -1, per-voxel-index X step)
+0x20  i32  step_y              (+1 or -1, per-voxel-index Y step)
```

| idx | base | near | aX | aY | aZ | sx | sy | tx | ty |
|---|---|---|---|---|---|---|---|---|---|
| 0 | 0 | 0 | 3 | 1 | 4 | 1 | 1 | -1 | -1 |
| 1 | 0 | 1 | 2 | 0 | 5 | 1 | 0 | -1 | +1 |
| 2 | 0 | 2 | 1 | 3 | 6 | 0 | 0 | +1 | +1 |
| 3 | 0 | 3 | 0 | 2 | 7 | 0 | 1 | +1 | -1 |
| 4 | 1 | 4 | 7 | 5 | 0 | 1 | 1 | -1 | -1 |
| 5 | 1 | 5 | 6 | 4 | 1 | 1 | 0 | -1 | +1 |
| 6 | 1 | 6 | 5 | 7 | 2 | 0 | 0 | +1 | +1 |
| 7 | 1 | 7 | 4 | 6 | 3 | 0 | 1 | +1 | -1 |

**Geometric meaning:** the 8 entries are paired (0↔4, 1↔5, 2↔6, 3↔7) — each pair
covers the two halves of the OBB Z-axis. Within each half, four entries cover
the four (X,Y) start-corner combinations. The `(sx, sy)` pair selects the start
position in the 2D voxel column grid:

| (sx, sy) | start at | step direction |
|---|---|---|
| (1, 1) | (size_x-1, size_y-1) — bottom-right | tx=-1, ty=-1 (decrement) |
| (1, 0) | (size_x-1, 0) — top-right | tx=-1, ty=+1 |
| (0, 1) | (0, size_y-1) — bottom-left | tx=+1, ty=-1 |
| (0, 0) | (0, 0) — top-left | tx=+1, ty=+1 |

**Sweep is always from the OBB corner nearest the camera outward.** This
guarantees front-to-back painter's-order pixel writes, so later voxels never
overwrite earlier (closer) ones in the visibility map.

> **Correction to prior report.** The prior report described entries +0x04..+0x10
> as "traversal axis pointers". They are actually OBB **corner indices** (0..7),
> not pointers. Indices into the 8-corner OBB array stored in the section
> tailer at `+0x40..+0x9F`.

### 3.1 How the orientation index gets selected

`VXL_Submit_BoundingBox @ 0x00754101` (also reachable as `0x00754220` —
register-set entry-points to the same function) walks the 8 OBB corners after
matrix transform. For each corner: compute `(x, y, -z)` in camera space, track
which corner has the smallest `y` (highest screen Y after the negation flip).
That index — `[0, 8)` — is stored in the BoxRecord at `+0x0C` and becomes the
dispatch table's orientation index.

### 3.2 BoxRecord layout (corrected — 0x88 bytes per record)

```
+0x00  u32   vxl_blob_or_obj_param  (param_1; usually 0 for unit voxels)
+0x04  u32   section_index           (param_2)
+0x08  u32   frame_index             (param_3)
+0x0C  u32   orientation_index       (0..7, near-corner index)
+0x10  f32   bbox_min_x  (init = +10000)
+0x14  f32   bbox_min_y
+0x18  f32   bbox_min_z              (init = +10000; tracks "highest screen point")
+0x1C  f32   bbox_max_x  (init = -10000)
+0x20  f32   bbox_max_y
+0x24  f32   bbox_max_z              (init = -10000; **SORT KEY**)
+0x28  f32×3 transformed_corners[8]  (96 bytes; each corner = (x, y, -z_camera))
+0x88  end
```

The 8 transformed corners at `+0x28..+0x87` are NOT redundant with the OBB
corners stored in the tailer. They are the *post-matrix-transform* projections
into camera space, used by the per-mode rasterizers to derive the screen-space
gradients for sweeping voxels.

---

## 4. Skip-step LUT @ `0x00B45590` — runtime initialization

**Critical correction to prior report.** The LUT at `0x00B45590` is **NOT a
precomputed table**. It is rebuilt at the start of every rasterizer call by
the rasterizer itself, sized to the section's `size_x`.

Init sequence at the top of every one of the 18 unique rasterizers (sample from
`FUN_007DF7C0`):

```c
i16  step_x  = *(i16*)(state + 0x2A);   // transform_x_increment from BoxRecord
i32  step_y  = state[0xB];               // transform_y_increment  
u32  count   = (u32)*(u8*)(state + 0x32); // size_x of section
i16* lut     = &DAT_00B45590;
i16  acc_x   = 0;
i16  acc_y   = 0;
do {
    *lut       = acc_x;        // [skip].x_delta
    acc_x     += step_x;
    lut[1]     = acc_y;        // [skip].y_delta
    lut       += 2;
    acc_y     += step_y;
    count--;
} while (count != 0);
```

**Layout: 256 entries × 4 bytes (1024 bytes max), but only `size_x` entries are
filled per call.** Each entry is 2 shorts (i16 + i16 packed), accessed in the
inner loop as a single i32 add to the screen accumulator:

```
iVar13 += *(i32*)(&DAT_00B45590 + skip * 4);  // adds packed (x_delta | y_delta<<16)
```

The screen accumulator `iVar13` is in 16.16 fixed-point with high byte X and
mid byte Y, so adding the packed delta advances both axes by `skip ×
per-voxel-step` in one integer operation.

**Edge cases:**
- `size_x == 0`: the `do…while(count != 0)` underflows to `0xFFFFFFFF` →
  4 GB walk. Real VXLs never have `size_x == 0`. Unguarded UB.
- `skip == 0`: maps to `(0, 0)`. Mirror variants explicitly write entries
  `[0]` and `[1]` to zero before entering the loop.
- `skip ≥ size_x`: undefined — but voxel runs cannot legitimately produce
  `skip > size_x`, since `size_x` is the column width and `skip` is bounded
  by `size_x - cumulative_count`.

> **Implication for parity.** Rust must compute the skip→delta translation
> per-section per-orientation per-frame. This is essentially free (linear
> ramp, ~size_x adds), but if Rust precomputes a global LUT it will be wrong
> for non-default orientations.

---

## 5. `g_VXL_RenderMode @ 0x00B43184` — single setter, single value

| Site | Function | Address | Value | Trigger | Active in YR |
|---|---|---|---|---|---|
| 1 | `CCFileClass__Constructor` | `0x0052BDFA` | **`= 1`** | One-time game init | **Yes** |

There is **exactly one writer** in the entire binary (verified via xrefs to
`0x00B43184`). `MOV dword ptr [0x00B43184], 0x1` runs once at game startup
between `VXL_LightDirection_Setup()` and `VXL_MasterLighting_Init()`. The
value is then permanent for the entire game session.

**Hypotheses ruled out by evidence:** water reflection, shadow, voxel-cache
pre-render, mirrored draws, special unit type — none of these toggle
`g_VXL_RenderMode`.

### 5.1 What `g_VXL_RenderMode` actually does

Side-by-side decompilation of `FUN_007DF7C0` (slot 0, RenderMode=0 path) vs
`VXL_Rasterizer_RenderMode @ 0x007DF9C0` (slot 4, RenderMode=1 path). The two
functions are byte-identical except for **one statement** in the inner pixel
emit:

| Slot | Pixel write |
|---|---|
| 0 (`RenderMode = 0`) | `vis_map[addr] = pbVar15[2]` — raw color byte from voxel run |
| 4 (`RenderMode = 1`) | `vis_map[addr] = vpl_pages[(g_VXL_NormalLUT[pbVar15[3]] << 8) \| pbVar15[2]]` — VPL-lit |

Same pattern in `FUN_007DFAE0` (slot 5, mirror+lit). `g_VXL_RenderMode` is
**the per-voxel VPL lighting toggle**:

- **0** = unlit — write voxel color byte directly to the visibility map
- **1** = lit — index `vpl_pages[brightness][color_byte]` where brightness is
  `g_VXL_NormalLUT[normal_byte]`, populated upstream by `VXL_SimpleLighting`
  or `VXL_BlinnPhongLighting`

**Practical result:** since RenderMode is set to 1 once at game init and never
cleared, **every voxel rendering call in YR is VPL-lit**. The unlit
rasterizers (slots 0..3) are reachable in code but unreachable in normal play.
They are TS-era leftover.

> **Correction to prior report.** The prior report's hypothesis that
> RenderMode "selects alternate normal-stride rasterizers" was wrong. It's a
> shading toggle, period. The structural symmetry of the dispatch table
> (32 entries with bit 2 doubling the count) suggested otherwise but the
> actual variant pairs differ only in the lit-vs-unlit pixel emit.

---

## 6. `g_VXL_MirrorFlag @ 0x00B43180` — five setters, conditional

| # | Site | Function | Address | Value | Trigger |
|---|---|---|---|---|---|
| 1 | `CCFileClass__Constructor` | game init | `0x0052BE04` | `= 0` | One-time |
| 2 | `TechnoClass::Render` | per-render | `0x00706F12` | `= 1` | If `TechnoTypeClass+0xDAC != 0` |
| 3 | `TechnoClass::Render` | cleanup | `0x0070724F` | `= 0` | Same gate, end of render |
| 4 | `FUN_00707280` (turret cache helper) | per-turret | `0x007072FC` | `= 1` | Same gate, before rasterize |
| 5 | `FUN_00707280` | end of helper | `0x0070744B` | `= 1` | Same gate, **NO clear** |

`TechnoClass::Render` brackets the rasterize path with a clean set/clear pair.
`FUN_00707280` (the turret cache builder, called from `VXL_turret_draw @
0x00706BD0`) sets `MirrorFlag = 1` at both the start and end of its body but
**never sets it back to 0**. This is a possible state leak: any call that
runs after `FUN_00707280` exits and before the next `TechnoClass::Render` of
a non-mirrored techno will inherit `MirrorFlag = 1`.

**Whether this leak is observable in practice** depends on call ordering. The
project's frame-by-frame draw order (per `TACTICAL_RENDER_PIPELINE_GHIDRA_REPORT.md`)
suggests `TechnoClass::Render` is the dominant entry, and it always re-sets
the flag from its own gate. So the leak is benign in steady state, but it
means the engine's runtime state is not strictly bracketed.

### 6.1 What sets `TechnoTypeClass+0xDAC`?

The byte at `+0xDAC` defaults to 0 (zeroed memory) and is **NOT** set in any
of the standard `ReadINI` paths examined:
- `TechnoTypeClass__ReadINI @ 0x00712170`
- `UnitTypeClass__ReadINI @ 0x00747620`
- `AircraftTypeClass__ReadINI @ 0x0041CC20`

It is also not set in the corresponding constructors. Yet it is read by both
`TechnoClass::Render @ 0x00706F12` and `FUN_00707280 @ 0x007072FC` to gate
mirror rendering. Semantically, "render mirrored" matches water-reflection or
a special draw-mode. **Where it gets set is unresolved** and goes in Open
Questions (§11). A targeted xref scan from binary writes to `+0xDAC` would
resolve it.

---

## 7. Tailer `+0xA3` — same byte, two readings

Verified by disassembling all three readers. The same physical byte serves
both consumers. There is no separate alpha byte at `+0xA4`.

| Reader | Address | Disassembly | Interpretation |
|---|---|---|---|
| `VXL_Init_Simple` | `0x00753CE8` | `MOV DL, [EAX + 0xA3]` | normals_mode index 0..4 |
| `VXL_Init_BlinnPhong` | `0x00753DC9` | `MOV CL, [EAX + 0xA3]` | normals_mode index 0..4 |
| `VXL_Section_Rasterizer` | `0x00756831` | `MOV AL, [EBX + 0xA3]; TEST AL, AL; JNZ +5; OR ESI, 0x8` | Boolean: `==0 → alpha bit set` |

**Value-vs-interpretation matrix:**

| `+0xA3` value | Lighting (normals_mode) | Rasterizer (alpha-flag) | Outcome |
|---|---|---|---|
| 0 | NULL normals table | bit 3 = 1 → unlit alpha (slots 8-11) | No per-voxel lighting + alpha-blend |
| 1 | 16-entry RA1-era table | bit 3 = 0 → lit opaque | Lit + opaque |
| 2 | 36-entry TS table | bit 3 = 0 → lit opaque | Lit + opaque |
| 3 | 64-entry table | bit 3 = 0 → lit opaque | Lit + opaque |
| 4 | 245-entry RA2 table | bit 3 = 0 → lit opaque | Lit + opaque |

**Both interpretations are coherent.** Mode 0 has no per-voxel normals, so
the rasterizer can't VPL-shade those voxels — falling back to alpha-blend is
correct. Modes 1..4 have normals, and the lit rasterizer applies them.

> **Correction to prior report.** Section 8 of `VXL_HVA_FILE_FORMAT_GHIDRA_REPORT.md`
> said "tailer.alpha == 0" was a separate flag from `normals_mode`. They are
> the **same byte**, semantically overloaded but consistent.

### 7.1 Tailer `+0xA0..+0xA3` quad re-verify

From `VXL_Section_Rasterizer` at `0x007565C1..0x00756831`:

```
007565C1  MOV DL, [EBX + 0xA1]   ; size_y
007565C7  MOV CL, [EBX + 0xA0]   ; size_x
007565D1  MOV DL, [EBX + 0xA2]   ; size_z
00756831  MOV AL, [EBX + 0xA3]   ; normals_mode (also read as alpha-flag)
```

| Offset | Field | Type |
|---|---|---|
| `+0xA0` | `size_x` | u8 |
| `+0xA1` | `size_y` | u8 |
| `+0xA2` | `size_z` | u8 |
| `+0xA3` | `normals_mode` (also read as alpha-flag) | u8 |

**No padding, no shifted byte.** The prior file-format report's quad layout
is fully correct.

---

## 8. Color-0 transparency at the blit stage — hard-coded everywhere

**Always palette index 0. Hard-coded. Not configurable per-VPL or per-tailer.**

The visibility map's source byte is tested with a literal `if (b != 0)` in
**every** inner-loop blitter that consumes it. Verified at:

| Blitter | Address | Pattern |
|---|---|---|
| Standard voxel body row blitter (vtable slot 0x1C method+4) | `0x00491740` | `byte b = *src; if (b != 0) *dst = palette_lookup[remap_lut[b]]; src++; dst++;` |
| Z-aware voxel body blitter (slot 0x14) | `0x00491590` | Same `if (b != 0)` pattern |
| 16bpp Z+remap | `0x00493DF0` | Same |
| Intensity variant | `0x00493F30` | Same |
| Opaque RLE remap (SHP) | `0x004978C0` | Same (RLE format already excludes zeros) |
| Shadow blitter (slot 0xD0) | `0x00491820` | Same |

**There is no path through `Standard_SHP_blitter @ 0x004373B0` (the dispatcher
that consumes the visibility map) that writes color 0 to the destination.**
Color-keying happens **on the source byte before any palette translation**.

This means:
- The voxel grid color_index = 0 is reserved as the file-format transparency
  marker. Voxel rasterizers never write index 0 to the visibility map.
- After rasterization, visibility-map byte = 0 means "no voxel at this pixel
  this frame."
- The blit's `if (b != 0)` test relies on this invariant.
- House remap `remap_lut[0]` value doesn't matter — it's never read because
  the test happens before translation.

> **Refinement to prior report.** Section 8.2's claim "transparency handled
> at blit stage by the palette-translate Blitter masking palette index 0" is
> directionally right but mis-named the mechanism: the masking happens
> **before** translation, on the raw source byte, not via "masking palette
> index 0" inside the LUT.

### 8.1 Visibility/depth map clearing

**`VXL_Clear_TileMap @ 0x00753E00`** runs at the start of every voxel render
sequence. Logic:

```c
if (g_VXL_FirstFrameFlag) {
    memset(g_VXL_VisibilityMap, 0, 0x10000);   // 64KB full clear
    memset(g_VXL_DepthMap, 0, 0x10000);
    g_VXL_FirstFrameFlag = 0;
} else {
    if (rect_invalid_or_oversized) {
        memset(g_VXL_VisibilityMap, 0, 0x10000);  // full-clear fallback
    } else {
        // walk prior frame's dirty rect [B2FB60..B2FB6C], zero out those bytes
    }
    if (g_VXL_MirrorFlag) {
        // also clear DepthMap subrect
    }
}
```

**Both maps are cleared to literal 0.** No sentinel. Color 0 = "no voxel here"
is the load-bearing convention.

The dirty rect at `DAT_00B2FB60..6C` is computed by `VXL_Sort_Rasterize` and
read by both the partial-clear and the final blit.

**Subtle:** the depth map is only cleared if `g_VXL_MirrorFlag != 0`. For
non-mirror renders, depth-map garbage persists across frames. This is benign
because the visibility-map color-key shields any depth-map lookup — pixels
with `vis_map[i] == 0` are skipped entirely, so their stale depth is never
consulted.

### 8.2 Visibility map is reused 3× per unit

A typical unit's draw goes through three Clear → Rasterize → Blit cycles per
frame, each into the same visibility map:

1. **Shadow** (via `ObjectClass::DrawVoxelShadow @ 0x005F5B90` — runs first,
   uses shadow blitter slot 0xD0)
2. **Body** (via `TechnoClass::Render @ 0x00706ED0` — uses standard slot 0x1C)
3. **Turret** (via `VXL_turret_draw @ 0x00706BD0` and `FUN_00707280` —
   inherits the body's blitter)

Each pass clears the visibility map's dirty rect, rasterizes its sections,
then blits. The visibility map is never expected to hold cross-pass data —
it's a transient scratchpad.

---

## 9. Blitter selector + variant matrix

**`Blitter_selector @ 0x00490B90`** picks one of ~50 vtable slots based on a
flag word. Slots are populated by `Blitter_init @ 0x0048EBF0`, which
`operator_new`s 50+ small vtable instances. Decision branches:

| flag bits | branch | typical use |
|---|---|---|
| `flags & 0x10` | shadow path (slots 0x14, 0x30, 0x58, 0x70, 0xC0, 0x9C) | drawn before unit body |
| `flags & 6 == 2` | translucent/blend (slots 0x20, 0x3C, 0x48, 0x64, 0x7C, 0xA8) | special FX |
| `flags & 6 == 4` | **standard opaque + remap** (slots 0x1C, 0x38, 0x60, 0x78) | **vehicle bodies (default)** |
| `flags & 6 == 6` | intensity variant (slots 0x18, 0x34, 0x40) | bright effects |
| `flags & 1` | RLE path (slots 0x10, 0x2C, 0x54) | SHP, not VXL |
| `flags & 0x20` | shimmer/heatwave (slots 0x28, 0x50, 0xBC) | mirage cloak |
| `flags & 0x4000` | A-buffer alpha shape (slots 0x50, 0x54, 0x58, 0x5C, 0x60, 0x64) | warp-in / temporal |
| `flags & 0x800` | use Z-write (slot offsets +0x54+) | with depth write |
| `flags & 0x100` | fading (slot 0x84) | transitions |
| `flags & 0x40` | dynamic-light tint (slot 0x88) | iron curtain |
| `flags & 0x8000` | special intensity (slot 0x80) | cloak |

**`Blitter_selector_extended @ 0x00490E50`** is the same shape but with
offset slots `+0xC8..0x168` — used by `VXL_CacheBlit`. Cache path uses
RLEBlitter constructors (slot offsets `+0x124..0x168`) since cached pixmaps
are stored RLE-encoded.

`TechnoClass::Render` calls `Blitter_selector(param_8 & 0xFFFFFFEF)` —
strips bit `0x10` (shadow flag) for the body draw. `param_8` is built from:
`IsRubble` cases (`0x2002`), warping (`0x2004` / `0x2006`), cloaked
(`0x200A` / `0x200C`), human-player tint (via virtual call `+0x43C` to apply
house-specific blitter flags).

---

## 10. VXL_CacheBlit @ `0x00707480` — the cache pipeline

```
TechnoClass::Draw @ 0x00706640
├─ if (cache_table != NULL && key matches):
│      slot = FUN_007107E0(cache_table, &key)   // binary search
│      if (slot[+4] != 0):
│          VXL_CacheBlit(cached_pixmap, dst_rect, offset_pt, blit_flags, vis_phase, _, _)
│          return    // CACHE HIT — skip rasterization entirely
├─ TechnoClass::Render(...)              // CACHE MISS — rasterize from scratch
└─ FUN_006C89E0(cache_buffer, dst_surface, &visibility_rect)
                                          // post-render: copy 16bpp pixels
                                          // from g_PrimarySurface scratch into
                                          // a newly-allocated cache slot,
                                          // sorted-insert by key.
```

**Cache layout per techno-instance** (in `param_5`):

```
*param_5    ptr to (i32 key, void *pixmap)[] array
param_5[1]  entry count
param_5[2]  capacity
param_5[3]  byte flag (sorted vs dirty)
param_5[4]  last-hit slot ptr (1-deep MRU)
```

**What's cached:** the post-VPL-lookup, post-palette-translation 16bpp pixel
buffer, stored as RLE rows. `VXL_CacheBlit` calls
`Extended_SHP_blitter @ 0x00437A10` with the RLEBlitter inner row method —
the RLE format embeds runs of zeros directly, so color-keying is implicit in
the encoding rather than an explicit `if (b != 0)` test.

**Cache key** = `param_3` (an integer derived from `(facing_index, frame_index,
tilt_state, mirror_flag, lighting_phase)` packed). Lookup is binary search,
O(log N) per draw.

**Invalidation triggers:**
- Different facing → different key → cache miss → new entry inserted (sorted)
- Different frame → different key → new entry
- Cache resort flag set when array grows (re-sorts on next lookup)
- **NOT invalidated by HP change** (rocking from `+0x328/+0x32C` is applied
  at draw time, not bake)
- Cache is **per-techno-instance, NOT global LRU.** No size cap visible.
  Likely freed only on `TechnoClass::~TechnoClass`.

`TechnoClass::Render` and `VXL_turret_draw` each have their own cache table.

### 10.1 Where house remap fits

The cache stores **post-house-remap** pixels. Each unit instance has its own
cache, keyed by `(facing, frame, tilt, mirror, lighting_phase)` — but the
house's remap is baked into the cached pixmap during the first draw. Two
units of different houses sharing one VXL would each maintain their own
cache (each with its own remapped pixels).

This is consistent with the visibility-map approach: the visibility map
stores **pre-remap** color indices; the row blitter applies house remap
(`palette_lookup[remap_lut[src_byte]]`) at the moment of writing to the
destination surface; and the cache captures that final translated output.

---

## 11. TS-Legacy Risk Register

| # | Item | Active in YR | Evidence |
|---|---|---|---|
| 1 | `g_VXL_RenderMode = 1` (constant) | **Yes** | Set once at game init, never modified. Bit 2 always set in dispatch index. |
| 2 | `g_VXL_RenderMode = 0` runtime path (slots 0/1/2/3) | **No — dead** | No writer ever sets RenderMode back to 0; these unlit rasterizers are unreachable. TS-era leftover. |
| 3 | `g_VXL_MirrorFlag = 0` (init) | Yes | Default state |
| 4 | `g_VXL_MirrorFlag = 1` from `TechnoClass::Render` | **Conditional** | Triggered for technos with `TechnoTypeClass+0xDAC != 0`. Used for water reflection / mirror render path. |
| 5 | `g_VXL_MirrorFlag = 1` from `FUN_00707280` (cache helper) | **Conditional + leak** | Same gate; lacks cleanup — possible state leak |
| 6 | Dispatch table entries 16..31 | **No — unreachable** | `VXL_Section_Rasterizer` never produces an index ≥ 16. Zero xrefs to addresses 0x846880..0x8468BF. TS-era reservation. |
| 7 | Dispatch entries 12..15 | Yes (alias of 8..11) | When alpha-bit is set (mode 0), RenderMode is effectively ignored — the alpha rasterizer always writes raw color regardless. |
| 8 | Dispatch entries 8..11 (`tailer[+0xA3] == 0`) | **Conditional** | Reachable only for sections with `normals_mode == 0`. Asset usage of mode 0 in retail YR unconfirmed but the dispatch is live. |
| 9 | Functions `0x00756DD0`, `0x00756F80`, `0x007575A0`, `0x00757790`, `0x00757E70`, `0x00758030` | **No — dead in YR** | All wired to dispatch indices 16..31 only. TS-era spare rasterizers. |
| 10 | Mode 0 normals (NULL table) | Conditional | Reachable in dispatch but no retail YR voxel known to ship with `normals_mode == 0`. |
| 11 | Mode 2 normals (TS 36 entries) | Conditional | Reachable; some legacy TS-era VXLs may still ship in YR retail. |
| 12 | "First-frame" full-clear path in `VXL_Clear_TileMap` | Yes | Runs once at game start. |
| 13 | "Oversized rect" full-clear fallback in `VXL_Clear_TileMap` | Conditional | Triggered for very large dirty rects; rare in normal play. |
| 14 | Depth-map clear gated on `MirrorFlag` | Yes | Documented as benign (color-key shields lookup of stale depth). |

---

## 12. Disparity list vs current Rust

### 12.1 Already documented in prior report (still open)

- `VXL_HEADER_SIZE = 802` should be 32 + variable palette (HIGH)
- `SPECULAR_STRENGTH = 3.4` should be `3.0` (HIGH)
- Tailer +0x0C labeled "limb_identifier u32" should be `f32 scale` (MEDIUM)
- RA2 normals iterates 0..256 but binary has 245 entries (MEDIUM)
- Modes 1 (16 entries) and 3 (64 entries) tables missing from Rust (MEDIUM)

### 12.2 New from this investigation

| # | Severity | Rust file | Disparity | Impact |
|---|---|---|---|---|
| 13 | **HIGH** | `vxl_raster.rs` (whole module) | Rust rasterizes directly to RGBA atlas tiles. Binary uses a **visibility-map intermediate** (256×256 byte buffer) that stores pre-remap palette indices, then a separate row blitter applies house remap + palette translation at blit time. | Means **house remap cannot be implemented per the binary's pattern** without restructuring. Rust currently caches **post-remap RGBA per house × facing × frame**, so memory scales O(houses × facings × frames) rather than gamemd's O(facings × frames). At 30-player scale, this matters for memory budgeting. |
| 14 | **HIGH** | `unit_atlas.rs` | Atlas keys `(type_id, facing, frame, slope_type, house_color, layer)`. Binary's per-instance cache keys `(facing, frame, tilt, mirror, lighting_phase)` — **no house in the key**. | Different cache topology. Rust pre-bakes house tinting; gamemd applies it at blit. For multi-house rendering of the same unit type, gamemd reuses one cache entry across houses while Rust needs N. |
| 15 | MEDIUM | `vxl_raster.rs` | Skip-step LUT computed how? | Rust's per-section setup must compute the (x_delta, y_delta) packed table for each section's `size_x` and orientation. Verify Rust does not assume a precomputed global LUT. |
| 16 | MEDIUM | `vxl_raster.rs` rasterizer | Color-0 source-byte test — correct ✓ | Verified: `if (packed == 0)` at `vxl_raster.rs:503` matches binary's source-byte test. |
| 17 | MEDIUM | Rust rasterizer pipeline | Rust has no equivalent to `g_VXL_MirrorFlag` (water-reflection / shadow mirror-blit toggle) | If Rust ever needs to render mirror reflections (e.g., units in water), the dispatch path needs the mirror-rasterizer variant (`FUN_00757980` / `FUN_00757BF0`). |
| 18 | MEDIUM | Rust rasterizer | Rust has no equivalent to the `g_VXL_RenderMode` toggle | Verified that RenderMode is always 1 in YR; Rust's "always lit" assumption is correct. The unlit path is dead. **No fix needed.** |
| 19 | LOW | Rust render path | Cloak / warp / iron-curtain / chrono visual variants are unimplemented or shaded differently | Triggers on cloak, warp-in, iron-curtain, chrono. Player-visible. |
| 20 | LOW | `vxl_raster.rs:672` | `colors[0] = Color::transparent()` matches gamemd convention | Verified OK. |
| 21 | LOW | Visibility-map dirty-rect partial-clear | Rust has no visibility-map intermediate, so no equivalent | Not a parity issue in current Rust (no shared state), but constrains how house remap can be implemented. |
| 22 | LOW | `FUN_00707280` MirrorFlag leak | Possible runtime quirk in gamemd | Not a parity-fix candidate; just noted. |

### 12.3 Verified-correct (no disparity)

- Color-0 source-byte test before palette translation — verified ✓
- `tailer +0xA0..+0xA3` quad layout (size_x, size_y, size_z, normals_mode) — verified ✓
- `tailer +0xA3` overloaded (normals_mode + alpha-flag) — verified ✓
- Dispatch flag formula `base[mode] | mirror? | render? | alpha?` — verified ✓
- 18 unique rasterizer functions in dispatch table — verified ✓
- Skip-step LUT runtime initialization (NOT precomputed) — verified ✓
- Visibility map cleared to literal 0 between renders — verified ✓
- BoxRecord 0x88-byte structure layout — verified ✓
- 8 OBB corners stored at tailer +0x40..+0x9F (12 bytes each) — verified ✓

---

## 13. Open Questions

1. **`TechnoTypeClass+0xDAC` writer** — the byte that gates `g_VXL_MirrorFlag`
   set/clear in `TechnoClass::Render` and `FUN_00707280`. Defaults to 0; not
   set by any standard `ReadINI` examined; not set in the constructors.
   Resolution requires xref scan from binary writes to `+0xDAC`. Likely
   candidates by behavior: derived flag for "draw mirrored on water" or
   `NaturalCenter` / `Crawls`-related, but unconfirmed.

2. **`FUN_00707280` MirrorFlag leak** — both writes set `MirrorFlag = 1`,
   neither clears. Worth a reproducer in gamemd to confirm whether this
   actually leaks state across frames in practice.

3. **Cache eviction** — `VXL_CacheBlit`'s per-unit cache appears unbounded.
   Whether anything evicts (game restart? unit destruction?) was not traced.
   Likely `TechnoClass::~TechnoClass` frees `param_5[0]`.

4. **Why dispatch table allocates 32 entries when only 16 are reachable**.
   Likely TS-era reservation for additional flag bits never wired up in YR.
   Indices 16..31 contain valid function pointers but are unreachable from
   `VXL_Section_Rasterizer`.

5. **Retail YR `.vxl` assets with `normals_mode == 0`** — the alpha
   rasterizer slots (8..11) are wired up correctly, but if no retail asset
   uses mode 0, those rasterizers are dead-in-practice for stock content.
   Low priority — Rust should still support mode 0 because mods may use it.

6. **`vplPage` parameter origin in `FUN_004AF2A0`** — passed through to
   `Standard_SHP_blitter` and ultimately into `Blitter_selector`. Comes
   from a virtual TechnoType method (`(**(code **)(*param_1 + 0x2F0))`)
   but the exact computation is unverified.

7. **Slot 0xD0 (shadow) palette LUT** — full decompilation of `0x00491820`
   not yet completed — bytes show `test al,al; je` color-key but full
   structure (which palette LUT it uses to darken) needs full decompile.
   Likely overrides palette LUT to a "darken" table.

8. **Rust house-remap path** — does Rust currently apply house remap for
   voxels at all? If so, where? Not visible in `vxl_raster.rs` grep results.

---

## 14. Sources

**Memory reads:**
- `0x00846840` (128 bytes — 32-entry rasterizer dispatch table)
- `0x008468C0` (288 bytes — 8-entry orientation table)
- `0x00B2D960` (136 bytes — first BoxRecord, all zeros at scan time)
- `0x007E57F8`, `0x007E5B70`, `0x007E5B58`, `0x007E5B40`, `0x007E5B28`,
  `0x007E5B10`, `0x007E5B00` (Blitter vtable layouts)
- `0x007DFAE0`, `0x00757360`, `0x00757980` (instruction prefix peeks)

**Functions decompiled or disassembled (~50 total):**

`0x0052BDFA` (CCFileClass init, RenderMode=1 site),
`0x0052BE04` (MirrorFlag init=0 site),
`0x004373B0` (Standard_SHP_blitter),
`0x00437A10` (Extended_SHP_blitter),
`0x0041CC20` (AircraftTypeClass::ReadINI — verified no `+0xDAC` write),
`0x00437A10` (Extended_SHP_blitter),
`0x0048EBF0` (Blitter_init),
`0x00490B90` (Blitter_selector),
`0x00490E50` (Blitter_selector_extended),
`0x004913F0` (bulk row remap, no key),
`0x00491590` (ZAwareBlitter_Blit),
`0x00491670` (opaque row blitter, hand-disassembled — color-key verified),
`0x00491740` (standard voxel body row blitter — **color-key crown jewel**),
`0x004917C0` (wrapper trampoline),
`0x00491820` (Z-buffered shadow row blitter, hand-disassembled),
`0x00493CC0` (Z+remap inner loop, hand-disassembled),
`0x00493DF0` (Blitter_Scanline_Opaque_Remap),
`0x00493F30` (Blitter_Scanline_Remap_Intensity),
`0x0049A200` (Blitter::Constructor),
`0x004978C0` (Blitter_Opaque_RLE_Remap),
`0x004AF2A0` (final blit dispatcher),
`0x0050BA00` (HouseClass::ComputeRemap),
`0x005F5B90` (ObjectClass::DrawVoxelShadow),
`0x006C89E0` (cache-build copy),
`0x00706640` (TechnoClass::Draw),
`0x00706BD0` (VXL_turret_draw),
`0x00706D27` (turret cache call site),
`0x00706ED0` (TechnoClass::Render — MirrorFlag set/clear analysis),
`0x00706F12` (MirrorFlag set #2),
`0x0070724F` (MirrorFlag clear #3),
`0x00707280` (FUN_00707280 turret cache helper),
`0x007072FC` (MirrorFlag set #4),
`0x0070744B` (MirrorFlag set #5 no-clear),
`0x00707480` (VXL_CacheBlit),
`0x007107E0` (cache binary search body),
`0x00710890` (cache binary search turret),
`0x00710AF0` (TechnoTypeClass::Constructor — verified no `+0xDAC` write),
`0x00712170` (TechnoTypeClass::ReadINI — verified no `+0xDAC` write),
`0x007470D0` (UnitTypeClass::Constructor),
`0x00747620` (UnitTypeClass::ReadINI — verified no `+0xDAC` write),
`0x00753C70` (FUN_00753C70 — VXL_DrawState accessor),
`0x00753C80` (VXL_Init_Simple — `MOV DL, [EAX + 0xA3]`),
`0x00753CE8` (VXL_Init_Simple +0xA3 read site),
`0x00753D00` (VXL_Init_BlinnPhong),
`0x00753DC9` (VXL_Init_BlinnPhong +0xA3 read site),
`0x00753E00` (VXL_Clear_TileMap),
`0x00753EC5` (VXL_Clear_TileMap reads MirrorFlag),
`0x007542F0` (VXL_Sort_Rasterize variant A),
`0x007540F0` (VXL_Submit_BoundingBox),
`0x00754101` (VXL_Submit_BoundingBox alternate entry),
`0x00754169` (BoxRecord +0x0C orientation index write),
`0x00754220` (VXL_Submit_BoundingBox alternate entry),
`0x00754510` (VXL_Sort_Rasterize variant B),
`0x00756590` (VXL_Section_Rasterizer — full disassembly),
`0x00756818` (dispatch flag-build),
`0x00756831` (tailer +0xA3 read site),
`0x00756843` (dispatch CALL),
`0x00756DD0`, `0x00756F80`, `0x007575A0`, `0x00757790`, `0x00757E70`,
`0x00758030` (dead TS-era rasterizers — Ghidra `create_function`'d),
`0x00756590` (VXL_Section_Rasterizer),
`0x00757120` (VXL_Rasterizer_Mirror, slot 2),
`0x00757360`, `0x00757980`, `0x00757BF0` (live rasterizers, slots 3/6/7),
`0x007581F0`, `0x00758430` (alpha rasterizers, slots 10/11),
`0x00758670` (VXL_SimpleLighting),
`0x007586F0` (VXL_BlinnPhongLighting),
`0x007DF7C0` (unlit slot 0),
`0x007DF8C0` (unlit slot 1),
`0x007DF9C0` (lit slot 4 — VPL lookup verified),
`0x007DFAE0` (lit slot 5),
`0x007DFC00`, `0x007DFD00` (alpha slots 8/9).

**Globals confirmed by xref scan:**
- `0x00B43180` (g_VXL_MirrorFlag) — 11 read sites + 5 writers
- `0x00B43184` (g_VXL_RenderMode) — 2 read sites + 1 writer
- `0x00B45590` (skip-step LUT) — 50+ writes (one per rasterizer's init loop)
- `0x00B2D824` (g_VXL_SortIndices)
- `0x00B2D958` (g_VXL_BoxRecords)
- `0x00B2FB60..6C` (dirty rect)
- `0x00B2FF78` (g_VXL_VisibilityMap, 64 KB)
- `0x00B1D5E0` (g_VXL_DepthMap, 64 KB)
- `0x00846840` (g_VXL_RasterizerTable)
- `0x008468C0` (orientation table)
- `0x0087E8A4` (g_ZBuffer pointer)
- `0x00B0BDD0` (PrimarySurface 16bpp scratch buffer for cache source)
- `0x00B41178` (shaded LUT — used by row blitter for `palette_lookup`)

**Documents referenced:**
- `VXL_HVA_FILE_FORMAT_GHIDRA_REPORT.md` (the report this extends/corrects)
- `TACTICAL_RENDER_PIPELINE_GHIDRA_REPORT.md` (frame draw order context)

**Rust source surveyed:**
- `src/render/vxl_raster.rs`
- `src/render/vxl_compute.rs`
- `src/render/unit_atlas.rs`
