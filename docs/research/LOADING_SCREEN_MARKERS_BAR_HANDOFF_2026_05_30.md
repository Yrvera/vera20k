# Loading-screen ② mmpb markers + ③ bar remap — handoff (2026-05-30)

Context: item ① (progress-bar backing color) is DONE — see
`LOADING_BAR_COLOR_VERIFICATION_2026_05_30.md` and `src/rules/color_scheme.rs`.
Items ② and ③ remain. Everything below was verified in Ghidra MCP this session,
so the next session can implement without redoing the RE.

## ② mmpb start-position markers — mechanism fully verified

Drawer: `FUN_00640A40`, called from the loading renderer `FUN_00552D60` at
`0x00553687` (the `g_GameMode != 0` / non-campaign branch — skirmish path; the
`g_GameMode == 0` branch is campaign and draws no markers). mmpb.shp string at
`0x00836DF4` (loaded at `0x00640e4e`).

### Projection (CONFIRMED — `FUN_006d62e0`, decompiled)
Input is a lepton coord: `lep = cell*256 + 128` for each axis.
```
sx = (lepX*60)/2 + (lepY*-60)/2          // = (lepX*60 - lepY*60)/2
sy = (lepX*30)/2 + (lepY*30)/2           // = (lepX*30 + lepY*30)/2
sx = (sx + (sx>>31 & 0xff)) >> 8          // arithmetic >>8 with negative-rounding bias
sy = (sy + (sy>>31 & 0xff)) >> 8
sx += 0x3c00                              // +15360
```
Then the consumers use `sx/60` (0x3c) and `sy/30` (0x1e). The `/60` and `/30` in
`FUN_00640A40` use the `0x88888889` magic (SAR 5 for /60, SAR 4 for /30, +sign).

### Projected playfield bbox (CONFIRMED — writer `FUN_0058b820`)
Iterate every cell where `MapClass__Is_Cell_In_Playfield` is true; project; take
min/max of `(sx/60)` and `(sy/30)`:
```
Scen[0x112c] = minX          Scen[0x1130] = minY
Scen[0x1134] = maxX - minX   Scen[0x1138] = maxY - minY   (the marker divisors)
```
So the divisors are the **projected extent in /60,/30 space** (NOT raw cells /
LocalSize). The preview surface `GenerateTerrainPreview` builds is
`((maxX-minX)*2) x (maxY-minY)` — note the **×2 on width**.

### Gates (CONFIRMED — disasm `0x00640f3a`–`0x00640f78`)
For slot `s` in `0..activeCount` (active via `FUN_0068bd80(s)`):
1. `Scen[0x1180 + s*4] != -1` (assigned house index).
2. mmpb SHP loaded (`[ESP+0x18] != 0`).
3. assigned house's ColorScheme `+0x30c != 0`, where scheme =
   `g_ColorSchemeArray[ HouseClass[assignedIdx] + 0x16054 ]`.
Start waypoint cell from `FUN_0068bcc0(&out, s)`. Marker = mmpb.shp frame 0 drawn
via `CC_Draw_Shape(... 0x400 ...)` with the assigned house's `+0x30c` convert
(so each marker is in that player's color).

### Marker placement formula (STRUCTURE confirmed; centering UNRESOLVED)
Per slot, `dx = waypointSx/60 - minX`, `dy = waypointSy/30 - minY`:
```
markerX = ((dx*1000000) / Scen[0x1134]) * scaleX / 1000000 + offsetX - 3
markerY = ((dy*1000000) / Scen[0x1138]) * scaleY / 1000000 + offsetY - 2
```
(`*1000000` via repeated `LEA *5` ×6 then `SHL 6`; `/1000000` via `0x431bde83`,
SAR 0x12. The `-3`/`-2` nudges match `MMPB_MARKER_NUDGE_X/Y`.)

The aspect-fit that produces `scaleX/scaleY/offsetX/offsetY` is computed at
`0x00640d36`–`0x00640df2`. High-level (from the decompile):
```
scale1000 = min(regionSizeY*1000 / previewH, regionSizeX*1000 / previewW)
xCenter   = (regionSizeX - previewW*scale1000/1000) / 2
yCenter   = (regionSizeY - previewH*scale1000/1000) / 2
```
where `previewW = (maxX-minX)*2`, `previewH = (maxY-minY)`, and region
size/origin come from the width-keyed rect (already in Rust as
`mmpb_region_rect`: 640 {385,270,200,200}; 800 {499,379,216,166}; 1024
{570,424,300,260} as origin_x,size_x,size_y,origin_y).

**UNRESOLVED:** the exact `scaleX/scaleY` and `offsetX/offsetY` register/stack
mapping (`[ESP+0x7c]`,`[ESP+0x80]`,`[ESP+0x84]`,`[ESP+0x88]`) is garbled by
mid-function stack reuse (a `PUSH 0x24` for `operator_new` at `0x00640da5`
shifts ESP between the stores and the reads). Pin these by careful frame
tracking or by single-stepping a trace before trusting the math — this is the
recurring direction-bug class; walk a concrete fixture (e.g. mp01t4 known start
cells) before shipping.

### Data plumbing needed (the real work)
`NativeLoadingScreenState` (`src/app_loading.rs`) currently has variant,
color_index, backing_rgb, progress, atlas. Need to add the projected markers.
- Compute the projected bbox over all playfield cells — map cells available
  after phase-1 (`MapLoadInitial.map_data`, `src/app_init.rs:142`).
- Resolved per-player (start cell, color) come from `assign_launch_starts` in
  `apply_skirmish_launch_session` (`src/app_skirmish.rs`) incl. Auto starts.
- Store projected marker (x,y,color) list in `NativeLoadingScreenState`; draw in
  `build_native_loading_instances` AFTER background, BEFORE the bar (and through
  the `RenderingProgressSink` repaint path).
- Add mmpb.shp frame 0 to the loading atlas (`loading_screen_chrome.rs`).

Acceptance: one marker per active player at its start waypoint, in that player's
color, over the loading art, before the bar.

## ③ bar-fill palette remap — NOT pinned, decode first

The filled bar is PROGBARM frame 0 drawn via `CC_Draw_Shape(..., 0x400, ...)`
through the player ColorScheme ConvertClass at `+0x30c` — a 16-shade palette
remap, NOT the current flat tint (`player_scheme_bar_rgb`, `app_loading.rs`).
- BLOCKER: `decompile_function 0x00555da0` (reached via `get_xrefs_from
  0x00643555`) times out — use `disassemble_function` / `get_assembly_context`.
- Decode ramp generator `FUN_0068c3b0` (builds the 16-shade convert into the
  ColorScheme `+0x30c` from the scheme's H,S,V via Cos/Sin + `FUN_00517440`).
  ColorScheme ctor `FUN_0068c710` stores raw H,S,V at `+0x308`/`+0x309`/`+0x30a`.
- Inspect the real PROGBARM.SHP frame-0 palette indices (decode with MPLS.PAL;
  there is a diagnostic test `zz_dump_progbarm_frame0_indices` in
  `loading_screen_chrome.rs`) to learn which indices fall in the house-color
  band 16–31 (those get remapped).
- Only implement once the per-shade remap reproduces with zero invention;
  otherwise leave the current tint with an honest "not parity-correct" comment.

## Note for whoever curates the findings JSON
`UI_PARITY_AUDIT_2026_05_29.findings.json` → `ls-progress-backing-shade-
approximated` has a WRONG `correction`/`reasoning` (claims fixed static HSV from
`&DAT_00887734`; says "ColorScheme+0x308 never appears"). It analyzed the RMG
bar (`FUN_00598960`/PROGBAR2), not the loading bar. The drift verdict stands but
the gamemd mechanism IS the player-dependent ColorScheme+0x308 HSV.
