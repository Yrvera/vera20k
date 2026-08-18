---
title: Skirmish Owner-Draw Asset Mapping (Focused Ghidra Research Report)
date: 2026-05-16
---

# Skirmish Owner-Draw Asset Mapping - Focused Ghidra Research Report

## Scope

This is a focused follow-up to
`SKIRMISH_SHELL_RETAIL_ASSETS_GHIDRA_REPORT.md`. It narrows the question to:

- which owner-draw callbacks are assigned to Skirmish shell controls;
- which retail PCX/SHP assets resolve in the current RA2/YR install;
- which assets are background/chrome versus flags/control pieces;
- what still needs verification before pixel-faithful menu reconstruction.

Active in YR: Yes for the owner-draw hook setup, Skirmish dialog procedure,
Skirmish map preview drawing, and asset loads described below.

Overall confidence: High for callback assignment and resolved asset dimensions;
Medium for exact per-control PCX composition because Ghidra does not expose all
callback functions as clean decompilable functions yet.

## 1. Executive Findings

1. `usai.pcx`, `rusi.pcx`, `yrii.pcx`, and `obsi.pcx` are **not backgrounds**.
   They are 47x23 8-bit PCX side/country flag/icon images. Skirmish's row flag
   controls are 32x12 dialog units, so these PCXs are the correct class of asset
   for row flag rendering.
2. `dbak6440.pcx` is the real background-class PCX found in the owner-draw pool:
   640x400, from `ra2.mix -> local.mix`.
3. `dlgsysa.pcx` and `dlgsysi.pcx` are 640x160 dialog-system/chrome PCXs, also
   from `ra2.mix -> local.mix`.
4. The numbered start marker used by the Skirmish/MP map thumbnail is verified
   as `STARTBUT.SHP`: 18x18, 1 frame, from `ra2md.mix -> localmd.mix`.
5. `mmpb.shp` is a separate 12x12, 1-frame map-preview/player-marker asset from
   `ra2md.mix -> localmd.mix`.
6. `BTN-MINS.SHP` and `BTN-PLUS.SHP` appear in the `Skirmish.cpp` binary string
   cluster, but they did **not** resolve through the current `AssetManager` archive
   stack and still have no direct Ghidra xrefs. Treat them as unresolved legacy or
   optional assets until a static-table/use-site trace proves otherwise.
7. The PCX files checked all include the standard 8-bit PCX VGA palette marker
   `0x0C` at `len - 769`, so they carry embedded palettes. This reduces the need
   to guess `DIALOG.PAL` for those PCXs, but exact gamemd conversion/blit palette
   handling remains open.
8. `MnScrnLCustomizeBattle.shp` exists and is large (`632x568`, 1 frame), but
   there is still no direct evidence that offline Skirmish dialog `0x102` uses
   it as the background. Its string is in a broad `UICmnds.cpp` shell table.

## 2. Owner-Draw Hook Setup

Primary function: `FUN_0060f9a0`.

Evidence:

- Referenced from `D:\ra2mdpost\ownrdraw.cpp`.
- Called by `FUN_00622b50` during shell dialog initialization.
- Reads the child window class name with `GetClassNameA`.
- Reads style with `GetWindowLongA(hwnd, GWL_STYLE)`.
- Installs a window procedure with `SetWindowLongA(hwnd, GWL_WNDPROC, 0x610ca0)`.
- Sends message `0x497` after hook setup.
- Calls `FUN_0061f210` once when `DAT_00ac48d4 == 0`; this is the PCX preload
  pass.

Callback assignment table recovered from `FUN_0060f9a0`:

| Win32 class/style condition | Callback address assigned | Local kind value | Skirmish relevance |
|---|---:|---:|---|
| `ScrollBar` | `0x0061C690` | `8` | scrollbar/dropdown support |
| `ListBox` | `0x00618D40` | `4` | dropdown/list portions |
| `ComboBox` | `0x00617250` | `3` | player/side/color/start/team combos |
| `msctls_trackbar32` | `0x0061D950` | `7` | game speed/credits/unit sliders |
| `msctls_progress32` | `0x0061D6D0` | `6` | generic progress controls, not central to Skirmish setup |
| `NewEdit` | `0x00614B30` | `1` | custom edit controls |
| `Edit` | `0x00614190` | `1` | local player name edit `0x6A0` |
| `Static` | `0x006153E0` | `2` | flag statics, labels, map preview placeholder |
| `SysTabControl32` | `0x006137D0` | `10` | shell tabs, not central to offline Skirmish |
| `Button`, style low bits `0x0B` | `0x00612B70` | `0` | owner-draw Start/Choose Map/Back buttons |
| `Button`, style low bits `0x03` | `0x006163A0` | `0` | auto-checkboxes |
| `Button`, style low bits `0x09` | `0x00616980` | `0` | button variant |
| `Button`, style low bits `0x07` | `0x0061E700` | `0` | button variant |

Tiny details:

- The same hook setup handles Skirmish, Host/Guest, WOL, options, and other shell
  dialogs. Skirmish is not a bespoke renderer.
- The Skirmish dialog resource already marks many controls owner-draw:
  `COMBOBOX` style `0x...0213` and `BUTTON` style low bits `0x0B`.
- The callback addresses are not all recognized by Ghidra as separate functions,
  but the dispatch table in `FUN_0060f9a0` is clear.

## 3. PCX Preload Pool

Function: `FUN_0061f210`.

Verified behavior:

- Called once from `FUN_0060f9a0`.
- Loads most PCX assets with `CDFileClass__Constructor(name, 2, 0)`.
- Loads `dlgsysa.pcx` through `FUN_006ba120`.
- Does not directly load `DIALOG.PAL`, `SHELL.PAL`, or `MAINBTTN.PAL` in this
  function.

Important implication:

- The owner-draw framework has a single broad PCX pool, not a Skirmish-only list.
  A filename in `FUN_0061f210` means "available to shell owner-draw," not
  necessarily "visible on the offline Skirmish screen."

## 4. Resolved Retail Asset Locations and Dimensions

Source: temporary archive probe using the repo `AssetManager` against
`<ra2-install>/`, with
`load_all_disk_mixes()`. No repo source was modified.

### 4.1 SHP Assets

| Asset | Source archive | Bytes | Format | Size | Frames | Evidence / meaning |
|---|---|---:|---|---:|---:|---|
| `STARTBUT.SHP` | `ra2md.mix -> localmd.mix` | `360` | SHP | `18x18` | `1` | Direct xref to `DrawStartPositions`; numbered start marker shape |
| `mmpb.shp` | `ra2md.mix -> localmd.mix` | `200` | SHP | `12x12` | `1` | Direct xref to `FUN_00640a40`; map-preview/player marker |
| `MnScrnLCustomizeBattle.shp` | `ra2md.mix -> #0xC93B27A0` | `360144` | SHP | `632x568` | `1` | Broad shell/WOL-style screen asset; not proven offline Skirmish background |
| `BTN-MINS.SHP` | missing | n/a | n/a | n/a | n/a | Binary string exists near `Skirmish.cpp`; no archive resolution |
| `BTN-PLUS.SHP` | missing | n/a | n/a | n/a | n/a | Binary string exists near `Skirmish.cpp`; no archive resolution |

### 4.2 Flag/Icon PCXs

All checked flag PCXs are 8-bit, 1-plane, RLE PCX files with `bytes_per_line=48`
and embedded VGA palette marker `0x0C`.

| Asset | Source archive | Bytes | Size | Meaning |
|---|---|---:|---:|---|
| `usai.pcx` | `ra2.mix -> local.mix` | `1443` | `47x23` | USA/Americans shell flag/icon |
| `rusi.pcx` | `ra2.mix -> local.mix` | `1333` | `47x23` | Russia shell flag/icon |
| `yrii.pcx` | `ra2md.mix -> localmd.mix` | `1385` | `47x23` | Yuri shell flag/icon |
| `obsi.pcx` | `ra2.mix -> local.mix` | `1295` | `47x23` | Observer shell flag/icon |

Finding:

- These are small icons. They are too small to be menu backgrounds and match the
  player-row flag/icon role much better.

### 4.3 Background and Dialog Chrome PCXs

| Asset | Source archive | Bytes | Size | Notes |
|---|---|---:|---:|---|
| `dbak6440.pcx` | `ra2.mix -> local.mix` | `269684` | `640x400` | background-class shell PCX |
| `dlgsysa.pcx` | `ra2.mix -> local.mix` | `18075` | `640x160` | dialog-system/chrome active variant candidate |
| `dlgsysi.pcx` | `ra2.mix -> local.mix` | `24356` | `640x160` | dialog-system/chrome inactive variant candidate |

Finding:

- If we need a real shell background/chrome starting point, `dbak6440.pcx` and
  `dlgsys*.pcx` are the relevant assets, not country PCXs like `usai.pcx`.

### 4.4 Combo/Edit/Button/Number PCXs

| Asset | Source archive | Bytes | Size | Likely owner-draw role |
|---|---|---:|---:|---|
| `cue_i.pcx` | `ra2md.mix -> localmd.mix` | `1282` | `18x18` | combo/edit/control piece |
| `cce_i.pcx` | `ra2md.mix -> localmd.mix` | `1282` | `18x18` | combo/edit/control piece |
| `cce_ir.pcx` | `ra2.mix -> local.mix` | `1298` | `18x18` | combo/edit right cap |
| `cce_il.pcx` | `ra2.mix -> local.mix` | `1298` | `18x18` | combo/edit left cap |
| `bud_ri24.pcx` | `ra2.mix -> local.mix` | `1422` | `23x24` | button up/down right piece, 24px family |
| `bud_mi24.pcx` | `ra2.mix -> local.mix` | `1596` | `36x24` | button middle piece, 24px family |
| `number0.pcx` | `ra2md.mix -> localmd.mix` | `1014` | `8x16` | digit glyph |
| `number9.pcx` | `ra2md.mix -> localmd.mix` | `1010` | `8x16` | digit glyph |
| `bst_uckg.pcx` | missing | n/a | n/a | binary string exists, archive resolution failed |
| `bst_chkg.pcx` | missing | n/a | n/a | binary string exists, archive resolution failed |

Finding:

- The control-piece assets are small tiled/capped fragments, not whole controls.
  Pixel-faithful reconstruction needs the callback composition rules, not just
  loading the images.

### 4.5 Palette Assets

| Asset | Source archive | Bytes | Format |
|---|---|---:|---|
| `DIALOG.PAL` | `ra2.mix -> local.mix` | `768` | 256-color PAL |
| `SHELL.PAL` | `ra2.mix -> cache.mix` | `768` | 256-color PAL |
| `SHELL2.PAL` | `ra2.mix -> cache.mix` | `768` | 256-color PAL |
| `MAINBTTN.PAL` | `ra2.mix -> local.mix` | `768` | 256-color PAL |

Palette caveat:

- These palettes exist, but direct xrefs from `DIALOG.PAL` / `SHELL.PAL` /
  `MAINBTTN.PAL` were not recovered in this pass.
- The PCX files checked include embedded VGA palettes. That is binary/asset
  evidence that a PCX renderer can decode them without a separate `.PAL`, but
  it does not prove gamemd ignores the shell `.PAL` files in every path.

## 5. Skirmish Map Preview SHP Details

### 5.1 `STARTBUT.SHP`

Function: `DrawStartPositions @ 0x00640710`.

Verified tiny details:

- Finds map thumbnail child control with `GetDlgItem(hwnd, 0x468)`.
- Draws only if `ScenarioClass+0x113C` is `> 0` and `< 9`.
- Reads start point X from `ScenarioClass+0x1140 + i*8`.
- Reads start point Y from `ScenarioClass+0x1144 + i*8`.
- Draw call is `CC_Draw_Shape(STARTBUT_SHP, 0, ...)`.
- Shape frame is always `0`.
- X position includes offset `-9`.
- Y position includes offset `-6`.
- It draws the text label `i + 1` after the shape.

### 5.2 `mmpb.shp`

Function: `FUN_00640a40`.

Verified tiny details:

- Walks playable map cells and projects cell coordinates to preview/screen space.
- Counts valid starts by testing `FUN_0068bd80(i)` for `i < 8`.
- Creates a temporary surface.
- Loads `mmpb.shp`.
- Iterates assigned start slots beginning at `ScenarioClass+0x1180`.
- Only draws if the start slot maps to a valid house and that house color scheme
  has a non-null value at `+0x30C`.
- Draws frame `0`.
- X expression includes `-3`.
- Y expression includes `-2`.

Interpretation:

- `STARTBUT.SHP` is the numbered available-start marker.
- `mmpb.shp` is a smaller marker tied to assigned player/house start rendering.

## 6. What This Means for Rebuilding the Client

Functional Skirmish/1v1 menu:

- We can build this now using the known dialog layout and data model.
- Required visual assets for basic parity are only the map preview and
  `STARTBUT.SHP` markers.

Asset-backed visual parity:

- Needs PCX decoding/rendering. The repo currently has `shp_file.rs` and
  `pal_file.rs`, but no PCX parser was found in `src/assets`.
- Needs an owner-draw composition layer for:
  - button cap/middle PCXs;
  - combo/edit cap/middle pieces;
  - checkboxes;
  - scrollbar/trackbar pieces;
  - flag static drawing.
- Should start with PCX images that resolved in retail archives:
  `dbak6440.pcx`, `dlgsys*.pcx`, `usai.pcx`/`rusi.pcx`/`yrii.pcx`,
  `cue_i.pcx`, `cce_i.pcx`, `bud_*`, and `number*.pcx`.

Do not start with:

- `BTN-MINS.SHP` / `BTN-PLUS.SHP`: unresolved in current archive stack.
- `MnScrnLCustomizeBattle.shp`: real asset, but not proven as offline Skirmish
  background.

## 7. Remaining Gaps

1. The callback bodies at `0x00612B70`, `0x00617250`, `0x006153E0`, etc. need
   function-boundary recovery or assembly-level tracing to extract exact PCX
   tiling/cropping rules.
2. The exact palette path for PCX blits remains partially unresolved. Asset data
   proves embedded PCX palettes exist; binary control code still needs tracing
   to prove whether gamemd uses embedded PCX palettes, shell `.PAL` files, or
   preconverted surfaces in each case.
3. `BTN-MINS.SHP` and `BTN-PLUS.SHP` need a static table/use-site trace. They
   may be legacy, optional, or loaded by a mechanism Ghidra did not xref.
4. `MnScrnLCustomizeBattle.shp` needs a direct draw xref or screenshot comparison
   before treating it as an offline Skirmish screen background.
5. A live screenshot comparison of the original offline Skirmish page would still
   be valuable to decide which owner-draw fragments are actually visible in the
   first viewport.

## Sources

Ghidra:

- `FUN_0060f9a0` - owner-draw hook setup.
- `FUN_0061f210` - owner-draw PCX preload pool.
- `FUN_00622b50` - shell dialog initialization/message path.
- `FUN_006ae3f0` - Skirmish dialog procedure.
- `FUN_006ae6e0` - Skirmish dialog initialization.
- `FUN_006acee0` - Skirmish command handler.
- `DrawStartPositions @ 0x00640710` - `STARTBUT.SHP` marker draw.
- `FUN_00640a40` - `mmpb.shp` map-preview/player marker draw.

Retail archive probe:

- Used `AssetManager::new(...)` plus `load_all_disk_mixes()`.
- Confirmed sources, sizes, and formats listed in section 4.
- Confirmed checked PCX files are 8-bit, 1-plane RLE PCX with embedded VGA
  palette marker.

Existing research:

- `docs/research/SKIRMISH_SHELL_RETAIL_ASSETS_GHIDRA_REPORT.md`
- `docs/research/SKIRMISH_SHELL_LAYOUT_ASSETS_GHIDRA_REPORT.md`
- `docs/research/SKIRMISH_START_POSITION_UX_GHIDRA_REPORT.md`
