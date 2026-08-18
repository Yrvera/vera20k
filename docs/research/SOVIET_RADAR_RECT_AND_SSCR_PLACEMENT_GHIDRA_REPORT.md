# SOVIET_RADAR_RECT_AND_SSCR_PLACEMENT_GHIDRA_REPORT

Status: PARTIAL - SSCR/MPSSCR placement and parent radar rect formulas were verified, but the tactical minimap content/inset rect inside the chrome was not proven by this scoped pass.

## Target Question

For Soviet sidebar/radar chrome, where do the `SSCR*` and `MPSSCRN*` SHPs draw, which rect globals feed those draw calls, when does the `+80` x-offset apply, and how do these positions relate to screen/sidebar panel coordinates?

## Non-goals

- Do not re-prove Soviet filename selection except where needed to identify which loaded SHP is drawn.
- Do not investigate ordinary build cameo layout, text layout, or power/credits placement.
- Do not inspect retail MIX membership or palette/ConvertClass selection.
- Do not modify Rust, INI files, Ghidra state, or existing published docs outside this report.

## Evidence Needed To Mark COMPLETE

- Read-only Ghidra decompile of `0x0072E920`, `0x0072E9F0`, `0x0072EAD0`, `0x0072EC70`, and `0x0072FC60`.
- Read-only Ghidra disassembly-range confirmation for handoff-critical x/y and branch claims.
- Proven relationship from `DAT_00b0fc1c` to screen width/height and right-panel coordinates.
- Proven `+80` offset predicate and which draw calls do or do not apply it.
- Proven minimap content/opening rect inside the radar chrome, or explicit uncertainty if that rect is outside the target functions.

## Stop Conditions

- Stop if Ghidra MCP read-only access is unavailable.
- Stop before any mutating Ghidra operation.
- Stop if target functions cannot be decompiled or inspected read-only.
- Stop before expanding into full `RadarClass` minimap rendering.

## Verified Findings

### 1. Parent radar rect `DAT_00b0fc1c` is computed by right-panel layout, not by the Soviet selector

Active in YR: Yes, for the right-panel/radar chrome path initialized by `RightPanel__Draw`.

`RightPanel__Draw @ 0x0072E450` initializes right-panel SHPs and calls `RightPanel__ComputeLayoutRects @ 0x0072EC70` when `DAT_00b0fbe0 == 0`. The rect allocated into `DAT_00b0fc1c` is:

- `x = 0` when `screen_w <= 1023`; otherwise `(screen_w - 800) / 2`.
- `y = 0` when `screen_h <= 767`; otherwise `(screen_h - 600) / 2`.
- `w = DAT_00b0fb50.width` when `screen_w == 640`; otherwise `DAT_00b0fa04.width`.
- `h = DAT_00b0fb50.height` when `screen_h == 480`; otherwise `DAT_00b0fa04.height`.

Evidence: `RightPanel__ComputeLayoutRects @ ram:0072EC70` decompile; Ghidra read-only disassembly range `0x0072EC70..0x0072EDAF` confirmed readable. Existing sibling docs identify `DAT_00b0fb50` / `DAT_00b0fa04` as `MNSCRNS.SHP` / `MNSCRNL.SHP` right-panel background/corner assets.

Concrete standard results:

| Screen | `DAT_00b0fc1c.x` | `DAT_00b0fc1c.y` |
|---|---:|---:|
| 640x480 | 0 | 0 |
| 800x600 | 0 | 0 |
| 1024x768 | 112 | 84 |

### 2. Static radar background draw applies `+80` x only for `screen_width > 799`

Active in YR: Yes, for the radar background draw path.

`RadarBackground @ 0x0072E920` copies `x = DAT_00b0fc1c[0]` and `y = DAT_00b0fc1c[1]`, then applies `x += 0x50` only when `g_ScreenWidth > 799`. It draws `g_RadarBackground_SHP` frame `0` with `CC_Draw_Shape(..., flags=0x400, z=1000, ...)`.

Evidence: `RadarBackground @ ram:0072E920` decompile; Ghidra read-only disassembly range `0x0072E920..0x0072E97F`.

Placement formula:

```text
draw_x = DAT_00b0fc1c.x + (screen_width > 799 ? 80 : 0)
draw_y = DAT_00b0fc1c.y
```

For standard sizes this means `640x480 -> (0,0)`, `800x600 -> (80,0)`, and `1024x768 -> (192,84)`.

### 3. Open-frame draw does not apply the `+80` x offset

Active in YR: Yes, for the radar/open transition right-panel path.

`FUN_0072E9F0 @ 0x0072E9F0` calls `Fill_Margins()`, then `RightPanel__Draw(0)`, then draws `g_RadarFrameOpen_SHP` frame `0` at exactly `{DAT_00b0fc1c[0], DAT_00b0fc1c[1]}`. No width branch and no `+0x50` adjustment appears in the decompile.

Evidence: `FUN_0072E9F0 @ ram:0072E9F0` decompile; Ghidra read-only disassembly range `0x0072E9F0..0x0072EA4F`.

For Soviet, prior selector proof maps this open-frame global to the first `SSCR*` loaded in `RadarBackground_SHPLoad`, i.e. the `SSCRBK*` branch for side `1`.

### 4. MPSSCRN movie draw also does not apply the `+80` x offset

Active in YR: Yes, for the radar transition movie draw path.

`FUN_0072EAD0 @ 0x0072EAD0` calls `Fill_Margins()`, then `RightPanel__Draw(0)`, then draws `g_MinimapMovie_SHP` frame `0` at exactly `{DAT_00b0fc1c[0], DAT_00b0fc1c[1]}`. No width branch and no `+0x50` adjustment appears in the decompile.

Evidence: `FUN_0072EAD0 @ ram:0072EAD0` decompile; Ghidra read-only disassembly range `0x0072EAD0..0x0072EB2F`.

For Soviet, prior selector proof maps `g_MinimapMovie_SHP` to `MPSSCRNS.SHP` when `screen_width == 640` and `MPSSCRNL.SHP` otherwise.

### 5. `LeftPanel__ComputeLayoutRects` is separate and does not place `SSCR*`/`MPSSCRN*`

Active in YR: Yes for left-panel/shell-side layout, but not the direct `SSCR*` placement path.

`LeftPanel__ComputeLayoutRects @ 0x0072FC60` computes left-panel/generic rects from `RADAR.SHP`, `TOP.SHP`, `BKGD*`, `CREDITS.SHP`, strips, button backgrounds, and lower-edge assets. It right-aligns the generic `RADAR.SHP` rect as `x = screen_w - RADAR.width`, `y = 0`, then stacks later rects vertically. It does not consume `DAT_00b0fc1c`, `g_RadarBackground_SHP`, `g_RadarFrameOpen_SHP`, or `g_MinimapMovie_SHP`.

Evidence: `LeftPanel__ComputeLayoutRects @ ram:0072FC60` decompile; Ghidra read-only disassembly range `0x0072FC60..0x0072FE5F`.

This keeps the Soviet `SSCR*` right-panel/radar placement separate from the generic non-Yuri left-panel `RADAR/BKGD*` layout.

## Implementation Handoff

1. Verified behavior -> Static Soviet radar background/chrome draw uses `DAT_00b0fc1c` plus `+80` only when `screen_width > 799`; open-frame and MPSSCRN movie draws use `DAT_00b0fc1c` directly. Rust delta -> `src/sidebar/mod.rs` and `src/render/sidebar_chrome.rs` currently model a fixed right sidebar/radar block from `screen_w - 168` and `radar.shp` frames, not the native `DAT_00b0fc1c`/`+80` split. Affected surface -> Soviet radar chrome alignment at 800+ and 1024+ shell/sidebar radar transitions. Acceptance scenario -> at 800x600, static `SSCRTMD`/background layer draws 80 px right of the open/movie origin, while `SSCRBKMD` and `MPSSCRNL` draw at origin. Proposed test -> `test_soviet_sscr_static_background_applies_plus_80_but_transition_layers_do_not`. Risk -> HIGH screenshot/alignment risk.

2. Verified behavior -> `DAT_00b0fc1c` origin is centered only when width >1023 or height >767, with 800x600 still origin `(0,0)`. Rust delta -> avoid deriving these shell/radar chrome origins from ordinary in-game sidebar right edge alone. Affected surface -> wide-screen right-panel/radar chrome placement. Acceptance scenario -> 1024x768 computes parent origin `(112,84)` and static background draw `(192,84)`. Proposed test -> `test_right_panel_radar_parent_rect_centers_800x600_band_at_1024x768`. Risk -> MEDIUM-HIGH for non-800 layouts.

3. Verified behavior -> `LeftPanel__ComputeLayoutRects` right-aligns generic `RADAR.SHP`/left-panel pieces but does not place `SSCR*` or `MPSSCRN*`. Rust delta -> keep `SSCR*` selector/placement separate from generic non-Yuri left-panel `RADAR/BKGD*` layout. Affected surface -> asset loader/layout ownership split. Acceptance scenario -> Soviet side can use `SSCR*` for right-panel radar transition while still using generic non-Yuri `RADAR/BKGD*` for left-panel loader. Proposed test -> `test_soviet_sscr_layout_is_separate_from_generic_left_panel_radar_rect`. Risk -> MEDIUM asset/layout conflation risk.

## Negative Facts / Do Not Do

- Do not apply the `+80` x-offset to `FUN_0072E9F0` open-frame draws; evidence `0x0072E9F0` decompile has no width branch and draws at `DAT_00b0fc1c`.
- Do not apply the `+80` x-offset to `FUN_0072EAD0` `MPSSCRN*` movie draws; evidence `0x0072EAD0` decompile has no width branch and draws at `DAT_00b0fc1c`.
- Do not model `DAT_00b0fc1c.x` as `screen_w - 168`; evidence `0x0072EC70` sets it to `0` for 640/800 and to centered-band margin only for width >1023.
- Do not treat `LeftPanel__ComputeLayoutRects @ 0x0072FC60` as the placer for `SSCR*`/`MPSSCRN*`; it computes generic left-panel rects from different SHP globals.
- Do not describe the static background offset as "800+ selector behavior"; the draw predicate is `g_ScreenWidth > 799`, while the selector predicate for small assets is exactly `g_ScreenWidth == 640`.

## Remaining Uncertainty

- The actual tactical minimap content/inset rect inside the `SSCR*` chrome was not proven here. The scoped functions place chrome/movie SHPs, not the minimap terrain surface.
- A direct consumer for `g_RadarFrameClose_SHP` / `SSCRA*` was not found in the named placement functions during this slot.
- Runtime SHP dimensions for `MNSCRNS/MNSCRNL` and the resulting `DAT_00b0fc1c.w/h` were not read from retail assets in this report.
- Palette/ConvertClass for these right-panel radar SHPs remains out of scope.

## Stale Doc Wording

- `docs/research/SIDEBAR_RADAR_POSITIONING.md`: replace "At 800+ resolution, the radar background is shifted right by 80px relative to the radar rect origin" with "Only `RadarBackground @ 0x0072E920` shifts its draw x by `+80` when `g_ScreenWidth > 799`; `FUN_0072E9F0` open-frame and `FUN_0072EAD0` MPSSCRN movie draws use `DAT_00b0fc1c` directly."
- `docs/research/SIDEBAR_RADAR_POSITIONING.md`: replace any simplified "radar rect x = sidebar right edge" wording with "`DAT_00b0fc1c.x` is `0` for widths up to `1023`, then `(screen_w - 800) / 2`; static radar background adds its own `+80` draw offset for widths over `799`."
