---
title: Skirmish Shell Retail Assets (Ghidra Research Report)
date: 2026-05-16
---

# Skirmish Shell Retail Assets - Ghidra Research Report

## Scope

This report answers which retail asset files `gamemd.exe` uses or references for
the Yuri's Revenge Skirmish / multiplayer setup shell, with emphasis on SHP,
PCX, and palette assets.

Active in YR: Yes for the Skirmish dialog procedure, map preview paint path,
owner-draw control framework, and `STARTBUT.SHP` start-marker draw path. Some
asset tables in this report are broader shell/WOL/UI pools and are marked as
such.

Overall confidence: Medium-high. The exact Skirmish dialog resource and start
marker asset are directly verified. The owner-draw PCX preload pool is directly
verified, but not every PCX in that pool is proven to be used by the offline
Skirmish page specifically.

## 1. Short Answer

The original Skirmish setup screen is not a PNG background. The base layout is
a compiled Win32 `DIALOGEX` resource in `gamemd.exe`, and the visual skin comes
from retail MIX-loaded assets plus owner-drawn Win32 controls.

Verified Skirmish/map-preview asset:

| Asset | Evidence | Confidence | Active in YR |
|---|---|---:|---|
| `STARTBUT.SHP` | String xref to `DrawStartPositions @ 0x00640710`; function loads a SHP pointer and calls `CC_Draw_Shape(... frame 0 ...)` for each start marker | High | Yes |

Verified owner-draw shell PCX asset pool:

| Asset family | Examples | Evidence | Confidence | Active in YR |
|---|---|---|---:|---|
| country/side flag PCXs | `usai.pcx`, `rusi.pcx`, `yrii.pcx`, `obsi.pcx`, etc. | `FUN_0061f210` calls `CDFileClass__Constructor(name, 2, 0)` for each | High for preload; Medium for exact Skirmish use | Yes, owner-draw shell path |
| dialog chrome PCXs | `dbak6440.pcx`, `dlgsysa.pcx`, `dlgsysi.pcx`, `bar_*.pcx`, `leftbar.pcx`, `rightbar.pcx`, `arrow_*.pcx` | same preload function | High for preload | Yes, owner-draw shell path |
| combo/list/edit PCXs | `cue_i.pcx`, `cce_i.pcx`, `cce_ir.pcx`, `cce_il.pcx`, `cud_i.pcx`, `ccd_i.pcx` | same preload function plus class dispatch for `ComboBox`, `ListBox`, `NewEdit`, `Edit` | High for preload; Medium for exact draw mapping | Yes, owner-draw shell path |
| scrollbar/trackbar PCXs | `sbgrip*.pcx`, `gsbgrip*.pcx`, `trakgrip.pcx`, `trof*.pcx`, `uparrow*.pcx`, `dnarrow*.pcx` | same preload function plus class dispatch for `ScrollBar`/trackbar | High for preload | Yes, owner-draw shell path |
| button/check/tab PCXs | `bud_*`, `bde_*`, `bue_*`, `bst_*`, `tab_*`, `bits_a.pcx`, `bits_i.pcx` | same preload function plus button/tab class dispatch | High for preload; Medium for per-control mapping | Yes, owner-draw shell path |
| number PCXs | `number0.pcx` through `number9.pcx` | same preload function | High for preload | Yes, owner-draw shell path |

Broader shell/UI SHP/PAL tables also exist, but they are not automatically
Skirmish-specific. Examples include `MnScrnLCustomizeBattle.shp/.PAL`,
`MultiplaySelection.shp/.PAL`, `MPYSCRNL.SHP`, `MPSSCRNL.SHP`, `MNBTTN.SHP`,
`DIALOG.PAL`, `SHELL.PAL`, and `MAINBTTN.PAL`.

## 2. No PNG / No Embedded Bitmap Background

Evidence from the prior PE resource extraction:

- Skirmish dialog resource ID is `0x102`.
- Template type is `DIALOGEX`, rect `0,0,533,369`, 72 controls.
- PE resource types include cursor, icon, menu, dialog, group cursor/icon, and
  version.
- No `RT_BITMAP` resources were present.

Finding:

- The window layout is in `gamemd.exe`.
- Background/control art is not embedded as a Windows bitmap resource.
- The binary strings and load paths show SHP, PCX, and PAL assets, not PNG.

## 3. Skirmish Paint Path

### 3.1 Dialog Procedure

Primary Skirmish dialog procedure: `FUN_006ae3f0`.

Relevant message behavior:

| Message / condition | Behavior |
|---|---|
| `0x497` | Calls `FUN_006ae6e0`, the Skirmish dialog initialization path |
| `WM_PAINT` (`0x0F`) | If `DAT_00ac1154 != 0`, gets dialog item `0x468`, calls a preview draw helper, then calls `DrawStartPositions` when that helper returns false |
| `WM_COMMAND` (`0x111`) | Calls `FUN_006acee0`, the Skirmish command/control handler |

Tiny details:

- The map thumbnail control is dialog item `0x468`.
- The paint path calls `ValidateRect` after drawing.
- The start markers are drawn from the shell dialog paint path, not from the
  tactical renderer.

### 3.2 Skirmish Initialization

Skirmish initialization function: `FUN_006ae6e0`.

Verified behavior relevant to assets/UX:

- Initializes local player edit control `0x6A0`.
- Populates seven AI/player slot combo boxes: `0x50B`, `0x50E`, `0x516`,
  `0x51A`, `0x51B`, `0x51C`, `0x51D`.
- Adds AI-slot choices with item-data values `-1`, `2`, `1`, `0`.
- Calls shared setup helpers for side, color, team, and start controls.
- If a selected AI slot maps to "closed/none" (`local_14 == -1`), it disables
  that row's side/color/team/start controls and writes `-2` defaults through the
  shared combo helpers.

Source evidence:

- `FUN_006ae6e0` references `D:\ra2mdpost\Skirmish.cpp`.
- It calls shared combo helpers later used by `FUN_006acee0` to collect side,
  color, team, and start selections.

### 3.3 Skirmish Command Handler

Skirmish command handler: `FUN_006acee0`.

Relevant control IDs:

| Control group | IDs | Behavior |
|---|---|---|
| AI/player slot combos | `0x50B`, `0x50E`, `0x516`, `0x51A`-`0x51D` | Updates slot state and refreshes controls |
| country/side combos | `0x6A1`, `0x510`, `0x513`, `0x514`, `0x51E`-`0x521` | Calls side-selection helpers |
| color combos | `0x6A2`, `0x522`-`0x528` | Handles color selection when notification code is `1` |
| start combos | `0x6A3`-`0x6A8`, `0x6AA`, `0x6AB` | Calls start-selection helper when notification code is `1` |
| Choose Map | `0x5AA` | Opens map selection and rebuilds preview state |
| Start Game | `0x617` | Validates player count/start constraints and stores settings |
| Back | `0x5C0` | Leaves dialog after storing/canceling |

Important start/location detail:

- On Start Game, the handler stores AI start locations into
  `DAT_00a8b2fc[i]` by calling `FUN_004e6030`.
- It stores the human player's start into `NodeNameTag+0x63` through the
  local player object construction path.

## 4. Start Marker Asset: `STARTBUT.SHP`

Function: `DrawStartPositions @ 0x00640710`.

String evidence:

- `STARTBUT.SHP` at `0x00836DE4`.
- Ghidra `batch_string_anchor_report` maps the string to
  `DrawStartPositions`.

Verified draw behavior:

- Validates the whole dialog rect.
- Looks up map preview child control `0x468`.
- Computes a scale from scenario visible-map bounds into the thumbnail rect.
- Loads a cached shape pointer guarded by `DAT_00ac4e90 & 1`.
- Draws only when `ScenarioClass+0x113C` is in the range `1..8`.
- Reads marker coordinates from:
  - X: `ScenarioClass+0x1140 + i*8`
  - Y: `ScenarioClass+0x1144 + i*8`
- Draws `STARTBUT.SHP`, frame `0`, using `CC_Draw_Shape`.
- Applies offsets:
  - X offset: `-9`
  - Y offset: `-6`
- Draws the label `i + 1` after the shape.

Why this matters:

- The numbered start symbols on the menu map preview are real SHP art.
- Rebuilding this faithfully needs `STARTBUT.SHP` plus text labels, not only
  egui text or tactical-map waypoints.

## 5. Map Preview / Player Marker Candidate: `mmpb.shp`

Function: `FUN_00640a40`.

String evidence:

- `mmpb.shp` at `0x00836DF4`.
- Ghidra maps it to `FUN_00640a40`.

Verified behavior:

- Walks playable map cells and projects them into screen/preview coordinates.
- Counts valid starts with `FUN_0068bd80(i)` for `i < 8`.
- Creates a temporary `DSurface`.
- Loads a SHP pointer via the `mmpb.shp` string xref.
- Iterates scenario start assignment slots beginning at
  `ScenarioClass+0x1180`.
- If a start slot has a valid assigned house and the house color scheme has
  usable data at `+0x30C`, draws SHP frame `0` through `CC_Draw_Shape`.
- Uses small offsets around the projected point:
  - X expression includes `-3`
  - Y expression includes `-2`

Confidence caveat:

- This function is map-preview related and uses `mmpb.shp`, but it is not the
  same immediate Skirmish dialog paint function as `DrawStartPositions`. Treat
  `mmpb.shp` as a verified map-preview/player-marker asset, not as the generic
  numbered start icon.

## 6. Owner-Draw Framework and PCX Pool

### 6.1 Control Hook Setup

Function: `FUN_0060f9a0`.

Evidence:

- References `D:\ra2mdpost\ownrdraw.cpp`.
- Called from `FUN_00622b50` during dialog initialization.
- Reads the Win32 class name and assigns an owner-draw/window-proc callback
  based on class and style.
- Calls `FUN_0061f210` once when `DAT_00ac48d4 == 0`; this is the PCX preload
  function.

Class dispatch found in `FUN_0060f9a0`:

| Win32 class/style | Callback address | Local kind value |
|---|---:|---:|
| `ScrollBar` | `0x0061C690` | `8` |
| `ListBox` | `0x00618D40` | `4` |
| `ComboBox` | `0x00617250` | `3` |
| `msctls_trackbar32` | `0x0061D950` | `7` |
| `msctls_progress32` | `0x0061D6D0` | `6` |
| `NewEdit` | `0x00614B30` | `1` |
| `Edit` | `0x00614190` | `1` |
| `Static` | `0x006153E0` | `2` |
| `SysTabControl32` | `0x006137D0` | `10` |
| `Button`, style low bits `0x0B` | `0x00612B70` | `0` |
| `Button`, style low bits `0x03` | `0x006163A0` | `0` |
| `Button`, style low bits `0x09` | `0x00616980` | `0` |
| `Button`, style low bits `0x07` | `0x0061E700` | `0` |

Why this matters:

- Skirmish controls are standard Win32 controls in the dialog resource, but
  `ownrdraw.cpp` hooks them and paints custom shell visuals.
- Visual parity needs both the resource/control rectangles and this PCX/SHP
  skinning layer.

### 6.2 PCX Preload Function

Function: `FUN_0061f210`.

Verified behavior:

- Calls `CDFileClass__Constructor(asset_name, 2, 0)` for most PCX files.
- Calls `FUN_006ba120("dlgsysa.pcx")` for one dialog-system PCX.
- The preload is guarded by `DAT_00ac48d4`, so it runs once per owner-draw
  shell lifetime.

Core PCX assets loaded by `FUN_0061f210`:

| Group | Assets |
|---|---|
| backdrop/dialog | `dbak6440.pcx`, `dlgsysi.pcx`, `dlgsysa.pcx` |
| bars/corners/arrows | `bar_ur.pcx`, `bar_ul.pcx`, `bar_lr.pcx`, `bar_ll.pcx`, `rightbar.pcx`, `leftbar.pcx`, `arrow_dd.pcx`, `arrow_du.pcx`, `arrow_ud.pcx`, `arrow_uu.pcx` |
| side/country icons | `yrii.pcx`, `obsi.pcx`, `rusi.pcx`, `lati.pcx`, `arbi.pcx`, `djbi.pcx`, `gbri.pcx`, `geri.pcx`, `frai.pcx`, `japi.pcx`, `usai.pcx`, `rani.pcx`, `nodi.pcx`, `gdii.pcx` |
| scroll/track | `trakgrip.pcx`, `sbgript.pcx`, `sbgripm.pcx`, `sbgripb.pcx`, `gsbgript.pcx`, `gsbgripm.pcx`, `gsbgripb.pcx`, `trofl.pcx`, `trofm.pcx`, `trofr.pcx` |
| scrollbar buttons/arrows | `sb_rel_d.pcx`, `sb_rel_u.pcx`, `sb_psh_d.pcx`, `sb_psh_u.pcx`, `uparrowr.pcx`, `dnarrowr.pcx`, `uparrowp.pcx`, `dnarrowp.pcx`, `guparrowr.pcx`, `gdnarrowr.pcx`, `guparrowp.pcx`, `gdnarrowp.pcx` |
| combo/edit/list control pieces | `cue_i.pcx`, `cce_i.pcx`, `cce_ir.pcx`, `cce_il.pcx`, `cud_i.pcx`, `ccd_i.pcx` |
| buttons/checks | `bud_ri24.pcx`, `bud_mi24.pcx`, `bud_li24.pcx`, `bde_ri24.pcx`, `bde_mi24.pcx`, `bde_li24.pcx`, `bue_ri24.pcx`, `bue_mi24.pcx`, `bue_li24.pcx`, same `bud/bde/bue` pieces for `30`, plus `bst_uckg.pcx`, `bst_chkg.pcx`, `bst_uchk.pcx`, `bst_chkd.pcx` |
| tabs | `tab_tlu.pcx`, `tab_tmu.pcx`, `tab_tru.pcx`, `tab_tld.pcx`, `tab_tmd.pcx`, `tab_trd.pcx`, `tab_ftl.pcx`, `tab_ftr.pcx`, `tab_ftm.pcx`, `tab_fbr.pcx`, `tab_fbl.pcx`, `tab_fbm.pcx`, `tab_fmr.pcx`, `tab_fml.pcx` |
| numbers | `number0.pcx` through `number9.pcx` |
| multiplayer score bars | `mpyscrnlbar01.pcx` through `mpyscrnlbar10.pcx`, `mpsscrnlbar01.pcx` through `mpsscrnlbar10.pcx`, `mpascrnlbar01.pcx` through `mpascrnlbar10.pcx` |
| WOL/rank/lobby icons | `pingr.pcx`, `pingy.pcx`, `pingg.pcx`, `wol*.pcx`, `wod*.pcx`, `wou*.pcx`, `private.pcx`, `corporal.pcx`, `sergeant.pcx`, `lieutena.pcx`, `major.pcx`, `colonel.pcx`, `stargen.pcx`, `briggenr.pcx`, `general.pcx`, `comchief.pcx`, `cooperat.pcx` |

Confidence caveat:

- The PCX preload function is a shared owner-draw shell pool. It proves the
  assets are loaded for the shell owner-draw framework. It does not prove that
  every WOL/rank/score PCX appears on the offline Skirmish screen.

## 7. SHP Assets in Nearby Shell/Skirmish Clusters

### 7.1 `Skirmish.cpp` Source Cluster

The `D:\ra2mdpost\Skirmish.cpp` string cluster contains:

- `BTN-MINS.SHP`
- `BTN-PLUS.SHP`

Ghidra result:

- `search_strings` finds both filenames.
- `find_undocumented_by_string` reports no direct code xrefs to either string.

Interpretation:

- These are Skirmish-source-cluster SHP filenames and are almost certainly
  intended for plus/minus Skirmish UI controls.
- Because Ghidra found no direct xrefs, this pass cannot yet prove the exact
  draw function or control IDs using them.

Confidence: Medium as Skirmish-associated filenames; Low for exact Skirmish
control placement until the static table/use site is resolved.

### 7.2 `ownrdraw.cpp` SHP Cluster

The owner-draw source cluster also includes SHP filenames:

- `SDWRNANM.SHP`
- `TMBTTN.SHP`
- `SMBTTN.SHP`
- `AMBTTN.SHP`
- `BTRANSBT.SHP`
- `BTRANSMD.SHP`
- `BTRANSTP.SHP`
- `RADTRANS.SHP`

Ghidra result:

- Strings are present near `D:\ra2mdpost\ownrdraw.cpp`.
- Direct string xrefs were not found for `SDWRNANM.SHP`, indicating table/static
  usage or incomplete xref recovery.

Interpretation:

- These are verified binary shell/owner-draw asset names.
- Exact per-control use remains unresolved.

## 8. Broader Shell Screen SHP/PAL Tables

The `D:\ra2mdpost\UICmnds.cpp` string cluster contains a broad UI/shell asset
registry. These names are real binary strings, but many are WOL, score screen,
faction select, loading, or sidebar assets rather than the offline Skirmish
dialog itself.

Examples:

| Group | Assets |
|---|---|
| WOL/shell screens | `WOLSoundOptions.shp/.PAL`, `WOLOptions.shp/.PAL`, `AutoLoginQuery.shp/.PAL`, `quickmatch.shp/.PAL`, `BuddyList.shp/.PAL`, `NewNick2.shp/.PAL`, `RegistrationScreen.shp/.PAL`, `LoginScreen.shp/.PAL`, `MultiplaySelection.shp/.PAL`, `MnScrnLCustomMatchLobby.shp/.pal`, `MnScrnLCoopGameSetup.shp/.PAL`, `MnScrnLCustomizeBattle.shp/.PAL` |
| faction select | `FSBKGDLG.SHP/.PAL`, `FSSLG.SHP/.PAL`, `FSALG.SHP/.PAL`, `FSSCRN.PAL`, `FSBKGDSM.SHP`, `FSSSM.SHP`, `FSBCLG.SHP`, `FSBCSM.SHP`, `FSASM.SHP` |
| sidebar/general UI | `RADARY.SHP`, `BKGDLGY.SHP`, `BKGDSMY.SHP`, `BKGDMDY.SHP`, `RENDCAP.SHP`, `BTTNBKGD.SHP`, `LENDCAP.SHP`, `LSPACER.SHP`, `SIDE2B.SHP`, `RADAR.SHP`, `TOP.SHP`, `CREDITS.SHP`, `BKGDLG.SHP`, `BKGDMD.SHP`, `BKGDSM.SHP`, `SIDEBTTN.SHP` |
| multiplayer score screens | `MPYSCRNL.SHP`, `MPSSCRNL.SHP`, `MPSSCRNS.SHP`, `MPASCRNL.SHP`, `MPASCRNS.SHP`, `SYCRAMD.SHP`, `SYCRTMD.SHP`, `SYCRBKMD.SHP`, `SSCRAMD.SHP`, `SSCRASM.SHP`, `SSCRTMD.SHP`, `SSCRTSM.SHP`, `SSCRBKMD.SHP`, `SSCRBKSM.SHP`, `ASCRAMD.SHP`, `ASCRASM.SHP`, `ASCRTMD.SHP`, `ASCRTSM.SHP`, `ASCRBKMD.SHP`, `ASCRBKSM.SHP` |
| common palettes | `DIALOG.PAL`, `DIALOGY.PAL`, `DIALOGN.PAL`, `SHELL.PAL`, `SHELL2.PAL`, `MAINBTTN.PAL`, `SDBTNANM.PAL`, `UIBKGD.PAL`, `UIBKGDY.PAL`, `SIDEBAR.PAL`, `MPLS*.PAL`, `MPYSCRN.PAL`, `MPSSCRN.PAL`, `MPASCRN.PAL` |

Important caution:

- `MnScrnLCustomizeBattle.shp/.PAL` sounds relevant by name, but the evidence
  here is only a broad `UICmnds.cpp` shell table. It is more likely tied to WOL
  "custom battle" screens than to the offline Skirmish `DIALOGEX 0x102` page.
  Do not use it as the first Skirmish background target without a screenshot or
  direct draw xref.

## 9. Current Rust Implementation Status

Observed current implementation:

- `src/ui/main_menu.rs` explicitly says it uses egui for a pragmatic client
  shell, not pixel-perfect RA2 chrome.
- `src/assets/shp_file.rs` and `src/assets/pal_file.rs` exist.
- No `pcx` parser or loader was found in `src/`.
- Current menu does not render the original owner-draw PCX shell chrome.
- Current menu does not render `STARTBUT.SHP` in the map thumbnail.

Parity implication:

- Functional 1v1 setup can proceed without these assets.
- Visual parity for the original Skirmish client requires a PCX parser/renderer
  and an owner-draw shell skin layer.
- The first concrete retail-art target should be `STARTBUT.SHP` for map start
  markers, because its Skirmish paint path is directly verified.
- The second target should be the country flag PCXs (`usai.pcx`, `rusi.pcx`,
  `yrii.pcx`, etc.) because Skirmish has transparent 32x12 static flag controls
  and the owner-draw framework preloads those PCXs.

## 10. Open Questions

1. `BTN-MINS.SHP` and `BTN-PLUS.SHP` need a static-table/use-site trace. The
   filenames are in the Skirmish source cluster, but direct Ghidra xrefs were
   not recovered.
2. The exact Skirmish background/chrome composition needs either a screenshot
   comparison or deeper callback disassembly for `Static`, `Button`, `ComboBox`,
   and `ListBox` owner-draw callbacks.
3. The exact palette used for every PCX control piece was not traced. The broad
   shell palette pool includes `DIALOG.PAL`, `SHELL.PAL`, `SHELL2.PAL`, and
   `MAINBTTN.PAL`, but per-control palette binding remains open.
4. `mmpb.shp` is verified in a map-preview/player-marker path, but its exact
   relationship to the Skirmish dialog thumbnail versus other preview surfaces
   should be confirmed with call-site tracing.
5. Retail archive source locations for each filename were not enumerated in
   this pass. The binary proves the names; a follow-up asset-manager scan should
   map each name to its MIX archive and frame dimensions.

## Sources

Ghidra functions decompiled/rechecked:

- `FUN_006ae3f0` - Skirmish dialog procedure.
- `FUN_006ae6e0` - Skirmish initialization.
- `FUN_006acee0` - Skirmish command/control handler.
- `DrawStartPositions @ 0x00640710` - map preview numbered start marker draw.
- `FUN_00640a40` - map-preview/player-marker rendering path using `mmpb.shp`.
- `FUN_0060f9a0` - owner-draw control hook setup.
- `FUN_0061f210` - owner-draw PCX preload pool.
- `FUN_00622b50` - shell dialog initialization/message handler that calls the
  owner-draw setup.

Binary string evidence:

- `STARTBUT.SHP` at `0x00836DE4`, xref to `DrawStartPositions`.
- `mmpb.shp` at `0x00836DF4`, xref to `FUN_00640a40`.
- `D:\ra2mdpost\Skirmish.cpp` at `0x0083FC4C`, xrefs to Skirmish functions.
- `BTN-MINS.SHP` at `0x0083FDB8`, no direct xrefs recovered.
- `BTN-PLUS.SHP` at `0x0083FDC8`, no direct xrefs recovered.
- `D:\ra2mdpost\ownrdraw.cpp` at `0x00833730`, xrefs to owner-draw functions.
- Owner-draw PCX strings at `0x00835xxx` / `0x00836xxx`, loaded by
  `FUN_0061f210`.
- `D:\ra2mdpost\UICmnds.cpp` at `0x00845598`, broad shell/UI asset table.

Prior docs referenced:

- `C:/Users/enok/Documents/ra2-rust-game-docs/SKIRMISH_SHELL_LAYOUT_ASSETS_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/SKIRMISH_START_POSITION_UX_GHIDRA_REPORT.md`
