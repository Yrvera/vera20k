# DirectDraw Runtime Pixel Format Masks - Ghidra Research Report

## Summary

Gamemd does not hardcode a single 16-bit sidebar/minimap framebuffer layout. In the standard DirectDraw video-mode path it requests a 16-bit display mode, creates the primary `DSurface`, asks DirectDraw for the primary surface descriptor, copies the descriptor pixel-format block, and derives the global channel shifts/losses from the returned red/green/blue masks.

The relevant runtime globals are:

| Global | Meaning | Evidence |
|---|---|---|
| `g_DD_RShift @ 0x008A0DD0` | count of trailing zero bits in the red mask | `DSurface__Constructor @ 0x004BA770`, loop after descriptor copy |
| `_g_DD_RLoss @ 0x008A0DD4` | `8 - red_bits`, derived by left-shifting the normalized mask until bit `0x80` is set | `0x004BA770` |
| `g_DD_BShift @ 0x008A0DD8` | count of trailing zero bits in the blue mask | `0x004BA770` |
| `_g_DD_BLoss @ 0x008A0DDC` | `8 - blue_bits` | `0x004BA770` |
| `g_DD_GShift @ 0x008A0DE0` | count of trailing zero bits in the green mask | `0x004BA770` |
| `_g_DD_GLoss @ 0x008A0DE4` | `8 - green_bits` | `0x004BA770` |

For the common RGB565 descriptor masks `R=0xF800`, `G=0x07E0`, `B=0x001F`, the derived values are `RShift=11`, `RLoss=3`, `GShift=5`, `GLoss=2`, `BShift=0`, `BLoss=3`, and gamemd classifies that as `DAT_008205D0 = 2`. For RGB555 masks `R=0x7C00`, `G=0x03E0`, `B=0x001F`, the derived values are `RShift=10`, `RLoss=3`, `GShift=5`, `GLoss=3`, `BShift=0`, `BLoss=3`, and gamemd classifies that as `DAT_008205D0 = 0`.

No attached in-process debugger capture was taken, so this report proves the active binary mechanism and supported classifications rather than a raw dump of the post-constructor globals. The current local `DDrawCompat-gamemd.log` separately records R5G6B5 selection for the enrolled wrapper run; that wrapper evidence is not a substitute for reading the globals inside `gamemd.exe`.

**2026-07-25 entry-point correction:** current live decompilation and assembly
show that `DSurface__Constructor` begins at `0x004BA770`. The previously cited
`0x004BA900` is an interior address in that function, not its entry point. The
descriptor-copy, shift/loss derivation, and classifier findings below remain
unchanged.

## Target and Non-Scope

Target: verify the active runtime DirectDraw pixel-format masks/channel shifts used by gamemd for the 16-bit surfaces relevant to sidebar, minimap/radar, text color packing, `AlphaBlendRect`, preview pack decode/write, and color remap tables.

Non-scope:

- Bink/movie surface internals except where a shared DirectDraw format getter/classification touches the same globals.
- Radar object priority, minimap inverse transform, radar transition asset lifecycle, terrain dirty pipeline, spy satellite, and tooltip glyph layout.
- Rust implementation or patches.
- Live debugger/watchpoint sampling of an already running retail process.

## Verified Binary Findings

1. **The standard video path requests a 16-bit DirectDraw display mode.**  
   Evidence: `FUN_00560BF0` calls `FUN_004A42F0` in its set-video-mode branch; `FUN_004A42F0 @ 0x004A42F0` logs `SetDisplayMode` and calls the DirectDraw vtable slot `+0x54` with the requested width, height, and bpp argument. The caller passes `0x10` for the bpp in the non-windowed mode-set branch. Active in standard YR: Yes; xrefs to `FUN_00560BF0` include `ScenarioClass__Start_Scenario @ 0x00683DF3` and other battle/control mode-change paths.

2. **The shifts/losses come from the primary surface descriptor, not from constants in sidebar or radar code.**  
   Evidence: `DSurface__Constructor @ 0x004BA770` creates the primary DirectDraw surface, calls the surface vtable `+0x58` to fill the surface descriptor, and copies eight dwords from descriptor offset `+0x48` into `DAT_008A0948..DAT_008A0964`. It then reads `DAT_008A0958`, `DAT_008A095C`, and `DAT_008A0960` as the red, green, and blue masks. Address `0x004BA900` lies inside this function.

3. **Mask-to-shift derivation is trailing-zero count bounded by 16.**  
   Evidence: in `DSurface__Constructor @ 0x004BA770`, each channel initializes its shift global to `0`, then repeatedly tests `(mask & 1)`, right-shifts by one, and increments the shift until a set bit appears or the shift reaches `0x10`. This is performed separately for red, green, and blue.

4. **Mask-to-loss derivation is `8 - channel_bits`, implemented by left-shifting until bit `0x80` is present.**  
   Evidence: after the trailing-zero loop, the same normalized mask is left-shifted until `(mask & 0x80) != 0`, incrementing the channel loss global until it reaches `8`. A 5-bit mask normalized to `0x1F` yields loss `3`; a 6-bit mask normalized to `0x3F` yields loss `2`.

5. **Gamemd recognizes RGB555 and RGB565 explicitly.**  
   Evidence: `DSurface__Constructor @ 0x004BA770` sets `DAT_008205D0 = -1`, then classifies known layouts. For `BShift=0`, `BLoss=3`, `GShift=5`, `GLoss=3`, `RShift=10`, `RLoss=3`, it sets `DAT_008205D0 = 0` and returns. For `BShift=0`, `BLoss=3`, `GShift=5`, `GLoss=2`, `RShift=11`, `RLoss=3`, it sets `DAT_008205D0 = _g_DD_GLoss`, i.e. `2`, and returns.

6. **The binary also has a third 16-bit layout classifier, but it is not the normal RGB555/RGB565 case.**  
   Evidence: the same classifier can set `DAT_008205D0 = 1` for a layout with `BShift=0`, `BLoss=2`, `GShift=6`, `GLoss=3`, `RShift=11`, `RLoss=3`, or a decompiler-equivalent adjacent branch. This proves callers must not assume only two possible classifier values, even though the common standard cases are RGB555/RGB565.

7. **`DAT_008205D0` is a shared surface-format classifier, not a direct RGB mask.**  
   Evidence: `FUN_004BBC90` returns `DAT_008205D0`. Xrefs include `LightConvertClass__Constructor @ 0x00555E46`, `BSurface__Constructor @ 0x005FF72E`, `BuildingClass_DrawBody`, `AnimClass__DrawIt`, `UnitClass__DrawPips`, and Bink-adjacent code. Consumers use it as a compact format category, while actual RGB packing/unpacking uses the six shift/loss globals.

8. **Primary display-color helper masks used by `AlphaBlendRect` are derived from the same shifts/losses, but stored separately.**  
   Evidence: `FUN_0060F9A0` initializes `DAT_00AC48B8`, `DAT_00AC48BA`, and `DAT_00AC48BC` once when `DAT_00AC48D4 == 0`. It starts from `0xFF`, right-shifts by each loss getter (`FUN_004BBC40`, `FUN_004BBC60`, `FUN_004BBC80`), then left-shifts by each shift getter (`FUN_004BBC30`, `FUN_004BBC50`, `FUN_004BBC70`). `AlphaBlendRect @ 0x00621B80` reads those three masks directly.

9. **The mask helper initialization is one-shot for the owner-draw/UI path.**  
   Evidence: `FUN_0060F9A0` guards mask construction with `if (DAT_00AC48D4 == 0)`, calls `FUN_0061F210`, then sets `DAT_00AC48D4 = 1`. Later owner-draw setup calls do not recompute the masks unless that global is reset elsewhere.

10. **Text color packing treats caller colors as `0x00BBGGRR` source RGB before DirectDraw packing.**  
    Evidence: `FUN_00621040 @ 0x00621040` extracts `param_5 & 0xFF` as red, `param_5 >> 8` as green, and `param_5 >> 16` as blue, then packs `((R >> RLoss) << RShift) | ((G >> GLoss) << GShift) | ((B >> BLoss) << BShift)` before calling the BitFont color setter.

11. **Generated radar/minimap terrain surfaces use the same shift/loss globals for 16-bit packing.**  
    Evidence: `RadarClass__GenerateTerrainSurface @ 0x006547C0` writes each generated secondary-surface pixel with red, green, and blue values clamped to `0xFF`, then packed through `_g_DD_*Loss` and `g_DD_*Shift`.

12. **Radar cell rendering uses the same globals for object dots and fog unpack/repack.**  
    Evidence: `RadarClass__RenderCellPixel @ 0x00655C50` packs object owner/color bytes through the globals; for fog, it reads a 16-bit secondary terrain pixel, unpacks through shifts/losses, halves channels with `>> 1`, then repacks through the same globals.

13. **Map preview decode/write also corroborates the RGB byte order and the same runtime packing.**  
    Evidence: `Straw__Constructor @ 0x00641B00` reads three decompressed bytes from `[PreviewPack]` as red, green, blue and packs them through `g_DD_*Loss/*Shift` before surface pixel write. Prior `PREVIEWPACK_DECODE_CHANNEL_ORDER_GHIDRA_REPORT.md` also records writer-side extraction at `0x006419D4..0x00641A2F`; this report spot-checked the load-side Ghidra path.

14. **Mode changes rebuild dependent color/remap tables after surface recreation.**  
    Evidence: `FUN_00560BF0` captures old shifts/losses through `FUN_004BBC40/30/60/50/80/70`, recreates primary/sidebar/other surfaces, then calls `FUN_00491100` with the old format tuple. `FUN_00491100 @ 0x00491100` iterates type-2 color remap tables and repacks their 256 entries from old shifts/losses to current `g_DD_*Loss/*Shift`, then stores `DAT_008A0DE8` and `DAT_008A0DEA` into each table and calls blitter initialization.

15. **Sidebar, hidden, alternate, tile, composite, radar primary, and radar secondary surfaces share the same global display-format contract.**  
    Evidence: `SidebarSurface_Create @ 0x005340??` constructs `DSurface` objects for hidden, composite, tile, sidebar, and alternate surfaces after the primary has established the global descriptor-derived format; `RadarClass__RebuildRadarSurfaces @ 0x00654650` constructs the radar primary display `DSurface` from the generated secondary `BSurface` dimensions. The color pack/unpack helpers do not carry per-surface masks; they read the process-global DirectDraw format globals.

## Active in Standard YR?

Active in standard YR: Yes for the mechanism.

- `FUN_00560BF0` is reached from scenario start and video/mode change paths and creates the DirectDraw primary and sidebar-related surfaces.
- `DSurface__Constructor @ 0x004BA770` is active for the primary DirectDraw surface and derives the global pixel-format values from the actual descriptor.
- `RadarClass__GenerateTerrainSurface`, `RadarClass__RenderCellPixel`, `FUN_00621040`, and `AlphaBlendRect` are active consumers in standard in-game UI/radar/sidebar paths.

Direct in-process RGB565 vs RGB555 value: deferred. The enrolled local wrapper log resolves its recorded run to R5G6B5, but the binary still requests 16 bpp and derives masks from DirectDraw's returned descriptor. Ghidra alone cannot prove which descriptor an arbitrary user's DirectDraw driver/wrapper returns. The common RGB565 case is explicitly recognized and would produce:

```text
RShift=11 RLoss=3
GShift=5  GLoss=2
BShift=0  BLoss=3
DAT_008205D0=2
```

The RGB555 case is explicitly recognized and would produce:

```text
RShift=10 RLoss=3
GShift=5  GLoss=3
BShift=0  BLoss=3
DAT_008205D0=0
```

## Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Native 16-bit colors are packed through runtime `R/G/B shift/loss`, not fixed RGBA8. | `DSurface__Constructor @ 0x004BA770`; consumers `0x00621040`, `0x006547C0`, `0x00655C50` | Mismatch/unchecked: Rust minimap/text use RGBA/GPU texture paths. | `src/render/minimap.rs`, `src/render/bit_font.rs`, retained sidebar surface/color helper surface | Add a native display-format packing abstraction for sidebar/minimap pixel parity, with selectable RGB565/RGB555 masks or runtime fixture masks. | `test_dd_pack_rgb565_and_rgb555_from_binary_shift_loss`: pack RGB `(255,255,255)`, `(255,0,0)`, `(0,255,0)`, `(0,0,255)`, and low-bit samples exactly as gamemd formulas. | Do not hardcode RGBA8 as the parity framebuffer. |
| RGB565 and RGB555 produce different green loss and red shift values; gamemd supports both. | Classifier in `DSurface__Constructor @ 0x004BA770` | Likely mismatch if Rust assumes only RGB565; `src/render/bit_font.rs` currently has RGB565-specific comments/tests around XOR color behavior. | `src/render/bit_font.rs`, color conversion helpers | Parameterize tests/helpers by shift/loss instead of embedding one layout except where a runtime fixture proves it. | `test_sidebar_dark_strip_differs_between_565_and_555_where_native_differs`: same source pixel under both masks yields expected different packed result. | Do not call RGB565 "the native format" without runtime descriptor proof. |
| `AlphaBlendRect` uses masks from `DAT_00AC48B8/BA/BC`, derived from the same shift/loss values. | `FUN_0060F9A0`; `AlphaBlendRect @ 0x00621B80` | Mismatch: Rust dark strips use RGBA alpha texture (`src/render/bit_font.rs`) and GPU blending. | `src/render/bit_font.rs`, `src/app_sidebar_build.rs`, future retained sidebar rasterizer | Compute dark strip pixels with packed mask math after choosing the native format. | `test_alpha_blend_rect_0xaf_uses_derived_masks_not_float_alpha`: representative RGB565 and RGB555 destination pixels match `((dst&mask)*0x50)>>8`. | Do not divide by 255, round, or blend in sRGB/linear float space. |
| Sidebar/minimap radar surfaces share global display-format packing; generated minimap is 16-bit primary/secondary surface data before sidebar blit. | `RadarClass__GenerateTerrainSurface @ 0x006547C0`; `RadarClass__RenderCellPixel @ 0x00655C50`; `RadarClass__RebuildRadarSurfaces @ 0x00654650` | Mismatch: Rust minimap builds/reuploads RGBA texture and draws as independent UI sprite. | `src/render/minimap.rs`, `src/app_render/build_instances.rs`, `src/app_render/draw_passes.rs` | Build generated minimap content as native-format pixels before composition into retained sidebar surface. | `test_minimap_fog_half_bright_uses_unpack_shift_loss_repack`: fogged terrain pixel halves unpacked channels and repacks exactly. | Do not implement fog/object dots as palette or alpha operations. |
| Text source color is `0x00BBGGRR`, then packed through runtime shifts/losses. | `FUN_00621040 @ 0x00621040` | Partial: Rust text color values may visually match in RGBA but are not proven through native quantization. | `src/render/sidebar_text.rs`, `src/app_sidebar_text.rs`, `src/render/bit_font.rs` | Sidebar text color tests should compare native packed value and final expanded display value for the chosen format. | `test_soviet_sidebar_yellow_text_packs_to_native_format`: `0x0000FFFF` source becomes RGB565 `0xFFE0` and RGB555 `0x7FE0`, depending on descriptor. | Do not treat `0x00BBGGRR` as already a framebuffer pixel. |
| Remap/blitter tables are repacked on video-mode changes from old shift/loss to new shift/loss. | `FUN_00560BF0`; `FUN_00491100 @ 0x00491100` | Unchecked for Rust; likely no mode-change repack equivalent because GPU assets are RGBA. | asset atlas/remap initialization, future native sidebar/minimap pixel fixtures | If supporting native packed surfaces, regenerate dependent remap tables when the emulated display format changes. | `test_display_format_change_rebuilds_native_color_tables`: switch RGB555 to RGB565 fixture and verify remap table entries update, not reused. | Do not cache 16-bit table entries across a format change. |

## Negative Facts / Do Not Do

- Do not hardcode RGB565 as gamemd's universal pixel format. The binary requests 16 bpp and derives the actual masks from the DirectDraw primary surface descriptor.
- Do not hardcode RGB555 either. It is explicitly supported, but not the only recognized 16-bit format.
- Do not use `DAT_008205D0` as a mask. It is a compact classifier; actual packing uses the six shift/loss globals.
- Do not treat `DAT_00AC48B8/BA/BC` as independent constants. They are derived from the same runtime shifts/losses and initialized once for the owner-draw/alpha helper path.
- Do not unpack to 8-bit RGB, blend with `/255` or floats, and repack unless exhaustive tests prove identical to the packed integer math for the chosen mask.
- Do not assume BSurface, DSurface, sidebar surface, and radar primary can have unrelated pixel formats in native in-game paths. The consumed helpers use process-global DirectDraw shifts/losses.
- Do not generalize the enrolled AMD/DDrawCompat/DXGI guard's final-pixel expansion to other display stacks. Binary source-pixel math is verified; capture/display conversion remains a separate, environment-scoped proof.

## Remaining Uncertainty

- Direct in-process descriptor sampling was not performed. A debugger/watchpoint on `DAT_008A0958/5C/60` or `g_DD_*Shift/Loss` after `DSurface__Constructor @ 0x004BA770` would confirm the exact post-constructor globals. The current local `DDrawCompat-gamemd.log` independently records R5G6B5 selection for the enrolled runtime.
- The exact semantic name of classifier value `1` remains bounded but not fully named; it is not needed for ordinary RGB555/RGB565 sidebar/minimap parity, but a compatibility layer should preserve it as a possible format fixture.
- Some non-sidebar consumers of `FUN_004BBC90` were not drained. They corroborate shared format classification, but their caller-specific visual effects are outside this slot.
- Final GPU/capture color-space parity in Rust remains implementation-verification work, not resolved by Ghidra alone.

## Stale-Doc Replacement Wording

Replace wording like:

> The game uses RGB565 for sidebar/minimap/AlphaBlendRect pixels.

With:

> Gamemd requests a 16-bit DirectDraw mode and derives `g_DD_R/G/BShift` and `_g_DD_R/G/BLoss` from the primary surface descriptor masks at runtime. RGB565 is explicitly recognized (`RShift=11, RLoss=3, GShift=5, GLoss=2, BShift=0, BLoss=3`, classifier `2`), and RGB555 is also explicitly recognized (`RShift=10, RLoss=3, GShift=5, GLoss=3, BShift=0, BLoss=3`, classifier `0`). Future pixel-parity work should use descriptor-derived shift/loss fixtures and only call the live install RGB565 or RGB555 after debugger/runtime sampling.

Replace wording like:

> AlphaBlendRect uses RGB565 channel masks.

With:

> `AlphaBlendRect` uses three 16-bit masks in `DAT_00AC48B8/BA/BC`, initialized from the current DirectDraw shift/loss globals by `FUN_0060F9A0`. The formula is packed-mask integer math over whatever 16-bit descriptor gamemd derived for the active display mode.

Replace wording like:

> Text color constants are framebuffer colors.

With:

> Shell/sidebar text helpers treat color constants as source RGB packed in `0x00BBGGRR` byte order, then quantize them through `g_DD_*Loss/*Shift` before rasterizing to the 16-bit surface.

## Status

COMPLETE for the binary mechanism: DirectDraw descriptor source, shift/loss derivation, RGB555/RGB565 classification, owner-draw mask construction, and active sidebar/minimap/text/alpha consumers were verified from Ghidra.

PARTIAL for direct in-process runtime identity: the enrolled wrapper log selects R5G6B5, but no attached debugger capture read the exact post-constructor globals inside `gamemd.exe`. The wrapper result and enrolled AMD/DDrawCompat/DXGI capture guard must not be generalized to other runtimes.

Sources:

- Ghidra decompile: `DSurface__Constructor @ 0x004BA770` (`0x004BA900` is an interior address)
- Ghidra decompile: `FUN_004A42F0`
- Ghidra decompile: `FUN_00560BF0`
- Ghidra decompile: `FUN_004BBC30`, `FUN_004BBC40`, `FUN_004BBC50`, `FUN_004BBC60`, `FUN_004BBC70`, `FUN_004BBC80`, `FUN_004BBC90`
- Ghidra decompile: `FUN_0060F9A0`
- Ghidra decompile: `AlphaBlendRect @ 0x00621B80`
- Ghidra decompile: `FUN_00621040`
- Ghidra decompile: `RadarClass__GenerateTerrainSurface @ 0x006547C0`
- Ghidra decompile: `RadarClass__RenderCellPixel @ 0x00655C50`
- Ghidra decompile: `RadarClass__RebuildRadarSurfaces @ 0x00654650`
- Ghidra decompile: `Straw__Constructor @ 0x00641B00`
- Prior docs referenced: `ALPHABLENDRECT_0xAF_DARK_STRIP_PIXEL_MATH_GHIDRA_REPORT.md`, `MINIMAP_GENERATED_PIXEL_COLOR_PIPELINE_GHIDRA_REPORT.md`, `RADAR_SURFACE_SIZING_ZOOM_SAMPLING_GHIDRA_REPORT.md`, `skirmish-ui/PREVIEWPACK_DECODE_CHANNEL_ORDER_GHIDRA_REPORT.md`
