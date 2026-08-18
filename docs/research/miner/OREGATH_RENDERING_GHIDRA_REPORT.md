# OREGATH.SHP Rendering — Ghidra Research Report

**Primary address:** `0x0073CEC0` (`UnitClass::DrawExtras`)
**CC_Draw_Shape:** `0x004AED70`
**Confidence:** HIGH — all findings verified from binary disassembly
**Active in YR:** Yes — always active for Harvester=yes units

## 1. Overview

OREGATH.SHP is the ore gathering arm animation overlay drawn on top of harvester units
while they actively mine. It is rendered via `CC_Draw_Shape` from `UnitClass::DrawExtras`
at `0x0073CEC0`. The SHP uses **15 animation frames per facing × 8 facings = 120 total
frames**. The animation plays at 1 frame per game tick (no INI Rate), with per-unit random
desync offsets.

## 2. Palette Finding (CORRECTED)

**OREGATH uses `anim.pal` (effect palette). The Rust implementation is correct.**

### Evidence chain from binary

CC_Draw_Shape at `0x004AED70` is `__fastcall`:
- **ECX** (param_1) = drawing surface — `[0x00887314]`
- **EDX** (param_2) = ConvertClass (palette converter) — `[0x0087f6c0]`

1. **CC_Draw_Shape call** (asm at `0x0073D276–0x0073D27D`):
   ```asm
   MOV EDX, dword ptr [0x0087f6c0]   ; EDX = ConvertClass (palette!)
   MOV ECX, dword ptr [0x00887314]   ; ECX = drawing surface
   CALL CC_Draw_Shape                 ; 0x004AED70
   ```

2. **`0x0087f6c0` is created from ANIM.PAL** during init at `0x0052BE63`:
   ```asm
   MOV ECX, 0x008260a0    ; "ANIM.PAL"
   PUSH 0x300             ; 768 bytes (256 × 3 RGB)
   CALL LoadFile
   ```
   The palette data is loaded, converted, and used to construct the ConvertClass
   stored at `0x0087f6c0`.

3. **Same ConvertClass is used by AnimClass::DrawIt** (`0x004232CE`, `0x004236F0`),
   confirming it is the anim/effect palette converter.

### Earlier analysis error

The initial analysis incorrectly identified ECX (the drawing surface at `0x00887314`)
as the palette. In `__fastcall` convention, ECX is the first parameter (surface),
while EDX is the second parameter (ConvertClass/palette).

## 3. CC_Draw_Shape Parameters for OREGATH

From the assembly at `0x0073D24E–0x0073D283`:

| Parameter | Value | Meaning |
|-----------|-------|---------|
| SHP data | `[0x00B1CF98]` (oregath.shp, lazy-loaded) | Cached SHP file pointer |
| Frame | `(unit+0x538 + g_CurrentFrameCounter) % 15 + (7 - ((facing>>12)+1>>1 & 7)) * 15` | Facing × 15 + anim frame |
| Position | Sin/cos offset from unit center | Arm placement |
| Clip rect | Current tactical viewport | Standard clip |
| Flags | `0x2A00` | See below |
| Remap | `0` (NULL) | No house color remap |
| Z-priority | `vtable+0x2EC(…) - 2` | Depth sorting |

### Flag 0x2A00 breakdown

| Bit | Hex | Meaning |
|-----|-----|---------|
| Center | `0x0200` | Subtract canvas_w/2, canvas_h/2 from position |
| Remap | `0x0800` | Enable remap table (but table = NULL → no-op) |
| Alt blitter | `0x2000` | Use alternative blitter path |
| Z-buffer | `0x0010` | Added at runtime when z_priority ≠ 0 |

**Blitter selection path** (`Blitter_selector` at `0x00490B90`):
- `0x10` set → z-buffer branch
- `0x3000 & flags` ≠ 0 → alt blitter sub-branch
- `0x800` set → return `*(this + 0x9C)` (z-buffer + alt + remap blitter)

## 4. Frame Layout and Positioning

### Frame calculation (verified from asm at `0x0073D25C`)

```
facing_index = 7 - (((body_facing >> 12) + 1) >> 1) & 7
anim_frame   = (unit+0x538 + g_CurrentFrameCounter) % 15
shp_frame    = facing_index * 15 + anim_frame
```

- `unit+0x538`: per-unit random desync offset (prevents lockstep animation)
- 15 frames per facing, 8 facings = 120 total SHP frames
- Facing reversal `7 - idx` converts from CW engine direction to CCW SHP layout

### Centering (CC_Draw_Shape flag 0x200)

```
pos_x = screen_x - canvas_width/2 + frame_x
pos_y = screen_y - canvas_height/2 + frame_y
```

Where `canvas_width/height` come from SHP header fields at offsets 2 and 4 (shorts),
and `frame_x/y` come from `SHP_frame_rect_getter` at `0x0069E7E0` (per-frame sub-rect
within canvas). The actual drawn size is `frame_width × frame_height` (per-frame).

**The Rust implementation handles this correctly** — offset = `frame_pos - canvas_center`,
sprite size = per-frame dimensions.

### Shadow frames

SHP files with >120 frames have shadows in the second half. The Rust code correctly
caps to 120 real frames (line 1131–1135 of sprite_atlas.rs).

## 5. Conditions to Draw

All must be true (from decompilation at `0x0073D0C3–0x0073D223`):

1. `UnitTypeClass+0xE0E != 0` — Harvester=yes
2. `unit+0x6D2 != 0` — actively harvesting flag
3. `locomotor->vfunc_0x80() == false` — NOT moving
4. `unit+0x278 == 0` — not deploying
5. `vtable+0x1D4() == false` — not cloaking
6. `vtable+0x1D8() == false` — not being chronoshifted

## 6. Current Rust Implementation Status

### Correct
- Frame layout: 15 × 8 = 120 ✓
- Per-unit desync offset ✓
- Animation rate: 1 frame per game tick (67ms) ✓
- Frame centering via canvas offset ✓
- Per-frame dimensions for rendering ✓
- Shadow frame exclusion ✓
- Arm offset (30 leptons rotated by body facing) ✓
- Harvest state gating ✓

### BUG (FIXED): Wrong render pass for OREGATH instances
- **Was**: `emit_harvest_overlay` pushed instances into the voxel unit instance list,
  which draws with `unit_atlas` textures. OREGATH UV coordinates reference `sprite_atlas`,
  so the result sampled garbage voxel data → "black white blocky mess".
- **Fix**: Route OREGATH instances to `shp_paged` (the SHP sprite draw list) so they
  draw with the correct `sprite_atlas` texture pages.

### Palette: CONFIRMED correct (anim.pal)
Verified from binary: `MOV EDX, [0x0087f6c0]` at `0x0073D276` loads a ConvertClass
created from `ANIM.PAL` (string at `0x8260a0`, loaded at `0x0052BE63`). The earlier
analysis claiming theater/unit palette was based on confusing ECX (surface) with EDX
(palette) in the `__fastcall` convention.

## 7. INI Keys

OREGATH has **no INI configuration**. All parameters are hardcoded:

| Parameter | Value | Source |
|-----------|-------|--------|
| SHP filename | `oregath.shp` | Hardcoded string |
| Frames per facing | 15 | Hardcoded |
| Facings | 8 | Hardcoded |
| Animation rate | 1 frame/tick | Hardcoded |
| Arm offset | 30 leptons | Hardcoded at `0x007F61D0` |
| Palette | anim.pal | Via ConvertClass at `0x0087f6c0` (EDX) |
| House remap | None (NULL) | Hardcoded |

## 8. Open Questions

None — all critical rendering parameters are verified from the binary.

## Sources

- **Ghidra addresses decompiled**: `0x0073CEC0` (DrawExtras), `0x004AED70` (CC_Draw_Shape),
  `0x00490B90` (Blitter_selector), `0x00544E70` (LightConvertClass creation),
  `0x005349C0` (theater palette loading), `0x0069E7E0` (SHP_frame_rect_getter)
- **Assembly verified**: `0x0073D24E–0x0073D283` (OREGATH draw call),
  `0x004B768B–0x004B7696` (ConvertClass init from `0x0088730C`)
- **Prior research**: `UNIT_DRAW_EXTRAS_REPORT.md` (verified and extended)
- **Rust code reviewed**: `src/render/sprite_atlas.rs`, `src/app_instances/units.rs`,
  `src/app_sim_tick.rs`
