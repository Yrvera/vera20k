# DirectDraw Live Pixel Format Runtime Sample - Ghidra Research Report

**Address(es):** `0x004BA9D0..0x004BAC27` descriptor-derived format globals/classifier, `0x004A42F0` DirectDraw mode set, `0x00621040` text packing, `0x006547C0` minimap terrain packing, `0x00655C50` minimap object/fog packing, `0x00621B80` AlphaBlendRect  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** local installed RA2/YR DirectDraw runtime pixel-format identity plus the gamemd consumers that make it minimap/sidebar-visible  
**Non-Scope:** generic minimap draw order, radar event composition, palette asset values, GPU screenshot/capture color-space conversion, non-DDrawCompat installs  
**Confidence:** High for binary mechanism and local DDrawCompat runtime format; Medium for direct `gamemd.exe` in-memory globals because the debugger server was not running  
**Active in YR:** Conditional. Active for standard YR when launched through the local install's `ddraw.dll`/DDrawCompat configuration; binary paths are active in standard YR, and the runtime log proves the local wrapper selected RGB565 resources in the recorded run.

## Working Notes

Target question: Which 16-bit DirectDraw pixel format does the active local user/runtime feed to gamemd for minimap/sidebar surfaces, and how does gamemd turn that into the `g_DD_*Shift/*Loss` packing contract?

Non-goals: Do not re-cover minimap terrain/object ordering, radar input, dirty queues, radar event draw order, or retail asset color tables except where a consumer proves pixel-format use.

Evidence needed to mark COMPLETE: runtime source evidence naming the local 16-bit resource format, Ghidra evidence that gamemd reads primary `GetSurfaceDesc` masks into global shifts/losses/classifier, and Ghidra evidence that minimap/sidebar consumers pack through those globals.

Stop conditions: stop after local runtime identity plus pixel-format consumers are proven; if no attached debugger is available, record debugger memory capture as a bounded uncertainty rather than expanding into launcher/process debugging.

## 1. Overview

The local installed DirectDraw runtime is not an unknown RGB555/RGB565 coin flip anymore. The literal current log, `DDrawCompat-gamemd.log`, from `<ra2-install>/` records the `gamemd.exe` process and the wrapper selecting `D3DDDIFMT_R5G6B5` for primary and plain resources under a directory config that forces 16-bit supported depth. Gamemd's own binary does not hardcode this; it requests 16 bpp, asks the primary DirectDraw surface for its descriptor, derives `g_DD_R/G/BShift` and `_g_DD_R/G/BLoss` from the returned masks, then minimap/sidebar/text/alpha consumers read those globals. This is wrapper-log evidence for the recorded run, not a direct in-process sample of those globals.

Therefore, for this local DDrawCompat-backed runtime, the minimap/sidebar native packed-pixel contract is RGB565:

| Channel | Mask | Shift | Loss |
|---|---:|---:|---:|
| R | `0xF800` | `11` | `3` |
| G | `0x07E0` | `5` | `2` |
| B | `0x001F` | `0` | `3` |

## 2. Runtime Evidence

| Runtime source | Finding | Active in YR | Evidence |
|---|---|---|---|
| Local wrapper file | The installed RA2/YR directory contains `ddraw.dll` plus `DDrawCompat.ini`; the wrapper is the DirectDraw implementation gamemd uses from this directory. | Conditional: active for local launches loading this directory DLL. | `<ra2-install>/ddraw.dll`; `DDrawCompat-gamemd.log:1-2` identifies `gamemd.exe` and says DDrawCompat loaded statically from that path. |
| Directory config | Supported depth is restricted/configured to 16-bit style output; render depth remains `app`. | Yes for this local config. | `DDrawCompat.ini:6-8`: `RenderColorDepth = app`, `SupportedDepthFormats = 16`, `DesktopColorDepth = 16`; log final config lines `28`, `43`, `58` confirm `DesktopColorDepth=16`, `RenderColorDepth=app`, `SupportedDepthFormats=D16`. |
| Runtime format selection | DDrawCompat selected RGB565, not RGB555, for the primary and plain resources in the logged run. | Yes for the logged local run; conditional on same wrapper/config path. | `DDrawCompat-gamemd.log:166-168`: `Using resource format: D3DDDIFMT_R5G6B5, plain, anymem -> vidmem`; `D3DDDIFMT_R5G6B5, primary, anymem -> vidmem`; `D3DDDIFMT_R5G6B5, plain, sysmem`. |
| Supported alternatives | The runtime supports both X1R5G5B5 and R5G6B5, but selection chose R5G6B5. | Yes for the logged local run. | `DDrawCompat-gamemd.log:91` lists `D3DDDIFMT_X1R5G5B5`; `DDrawCompat-gamemd.log:94` lists `D3DDDIFMT_R5G6B5`; lines `166-168` choose R5G6B5. |
| Debugger memory sample | No attached debugger server was available, so `g_DD_*` live memory could not be read directly this session. | Conditional; not a negative against the log. | `debugger_read_memory` for `0x008A0DD0` returned `Debugger server not running at http://127.0.0.1:8099`. |

## 3. Binary Mechanism

### 3.1 Mode and Descriptor Source

`FUN_004A42F0 @ 0x004A42F0` is the DirectDraw display-mode path. Prior report evidence and this spot-check show the caller passes `0x10` bpp on the normal mode-set branch. `DSurface__Constructor` then creates a DirectDraw surface and calls the surface vtable slot `+0x58`, the descriptor getter, before reading the pixel-format masks from descriptor-derived globals.

The material descriptor-to-global assembly lives in the same constructor region around `0x004BA9D0..0x004BAC27`:

| Operation | Evidence | Active in YR |
|---|---|---|
| Red mask source | `0x004BA9E?` loop stores `g_DD_RShift @ 0x008A0DD0`; surrounding code reads the red mask before `0x004BAA00`. | Yes, constructor path active for DirectDraw surfaces. |
| Red loss | `0x004BAA09` stores `g_DD_RLoss @ 0x008A0DD4` after the left-shift-until-`0x80` loop. | Yes. |
| Green mask source | `0x004BAA00` loads `0x008A095C`; `0x004BAA0F` initializes `g_DD_GShift`; `0x004BAA27` writes `0x008A0DE0`; `0x004BAA15` initializes `g_DD_GLoss`. | Yes. |
| Blue mask source | `0x004BAA39` reads `0x008A0960`; `0x004BAA83` writes `g_DD_BLoss @ 0x008A0DDC`. | Yes. |
| Classifier default | `0x004BAA89` writes `DAT_008205D0 = -1` before known-format classification. | Yes. |

The shift loop is a trailing-zero count capped at `0x10`. The loss loop left-shifts the normalized mask until bit `0x80` is set, capped at `8`; this implements `8 - channel_bits`. A 5-bit component gives loss `3`; a 6-bit component gives loss `2`.

### 3.2 RGB555 and RGB565 Branches

| Format | Branch evidence | Result | Active in YR |
|---|---|---|---|
| RGB555 / X1R5G5B5 | `0x004BAB4E` checks `GShift == 5`; `0x004BAB55`/`0x004BAB5B` require blue/red losses consistent with `3`; `0x004BAB5F` checks `RShift == 0xA`; `0x004BAB7D` writes `DAT_008205D0 = 0`. | `RShift=10 RLoss=3 GShift=5 GLoss=3 BShift=0 BLoss=3`, classifier `0`. | Supported by binary; not selected by the local DDrawCompat log. |
| RGB565 / R5G6B5 | `0x004BABBE` requires `ESI == 2` (`GLoss=2`); `0x004BABC3` checks `RShift == 0xB`; `0x004BABCC` requires red loss `3`; `0x004BABD9` writes `DAT_008205D0 = ESI`, therefore `2`. | `RShift=11 RLoss=3 GShift=5 GLoss=2 BShift=0 BLoss=3`, classifier `2`. | Yes for this local runtime, because DDrawCompat chose `D3DDDIFMT_R5G6B5`. |
| Third 16-bit classifier | `0x004BAB99..0x004BABBc` and `0x004BABF1..0x004BAC07` can write classifier `1` for another supported 16-bit layout. | Classifier `1`, not RGB555/RGB565. | Binary-supported, not selected by local log; out of scope for ordinary local minimap/sidebar parity. |

### 3.3 Consumers

| Consumer | Pixel-format use | Evidence | Active in YR |
|---|---|---|---|
| Sidebar/shell text wrapper | Treats source color as RGB bytes in `0x00BBGGRR`, then packs with `((R >> RLoss) << RShift) | ((G >> GLoss) << GShift) | ((B >> BLoss) << BShift)`. | `FUN_00621040 @ 0x00621040`; instructions at `0x0062104E+` read `g_DD_GLoss`, `g_DD_GShift`, `g_DD_BLoss`, `g_DD_BShift`, `g_DD_RLoss`, `g_DD_RShift`. | Yes. |
| Minimap generated terrain surface | Writes 16-bit terrain pixels through the same shift/loss globals after clamping RGB channels to `0xFF`. | `RadarClass__GenerateTerrainSurface @ 0x006547C0`; decompile writes packed `ushort` using `_g_DD_R/G/BLoss` and `g_DD_R/G/BShift`. | Yes in ordinary in-game radar generation. |
| Minimap object dots and fog | Object dots pack house/disguise RGB bytes through the globals; fog unpacks a 16-bit terrain pixel through shifts/losses, halves channels, then repacks. | `RadarClass__RenderCellPixel @ 0x00655C50`; object block reads `House+0x56F9..0x56FB`; fog block reads from secondary surface and repacks via globals. | Yes. |
| AlphaBlendRect | Uses three derived masks in `DAT_00AC48B8/BA/BC`, not an RGBA/floating alpha path. | `FUN_0060F9A0` builds masks from getter helpers; `AlphaBlendRect @ 0x00621B80` applies packed-mask integer blend. | Yes for sidebar dark strips and related owner-draw UI. |
| Radar primary surface rebuild | Radar secondary content is copied into a `DSurface` created after the display-format globals exist; no per-radar alternate mask is carried. | `RadarClass__RebuildRadarSurfaces @ 0x00654650` constructs `DSurface` for radar display surface after `GenerateTerrainSurface`. | Yes. |

## 4. INI / Runtime Config Keys

No RA2/YR gameplay INI key controls RGB555 vs RGB565. The relevant local runtime configuration is DDrawCompat-side:

| File | Key | Value | Effect |
|---|---|---|---|
| `DDrawCompat.ini` | `RenderColorDepth` | `app` | Lets app's requested color depth drive render depth. |
| `DDrawCompat.ini` | `SupportedDepthFormats` | `16` | Restricts advertised depth formats to 16-bit. Log normalizes this to `D16`. |
| `DDrawCompat.ini` | `DesktopColorDepth` | `16` | Forces the emulated desktop depth seen by the app/wrapper. |

## 5. Current Rust Implementation Status

| Rust surface | Current evidence | Delta |
|---|---|---|
| `src/render/minimap.rs` | `base_terrain_rgba`, `rgba_scratch`, `BatchTexture`, and `write_texture` manage a 200x200 RGBA GPU texture. | Drift for native packed-surface parity; gamemd builds native 16-bit pixels using runtime shift/loss. |
| `src/render/minimap_helpers.rs` | Comments and helpers define RGBA fallback terrain/overlay/dimming colors. | Drift where helpers bypass native quantization/unpack/repack, especially fog and object dots. |
| `src/render/bit_font.rs` | Missing-glyph fallback explicitly models RGB565 `color ^= 0x5555`; dark strip is a 1x1 RGBA texture with alpha. | RGB565 assumption is now correct for the local DDrawCompat runtime, but the mechanism is still native packed math, not RGBA alpha. Tests should still support descriptor fixtures. |
| `src/app_skirmish_shell_render.rs` | Owner-draw comments note `0x00BBGGRR` and runtime conversion is not represented in the current RGBA path. | Needs native-format test coverage for source RGB to packed16 conversion if targeting pixel parity. |
| `src/render/sidebar_chrome.rs` | Chrome art is pre-rendered to RGBA atlas textures. | Final visible color parity remains unchecked against RGB565 packed quantization and native blit path. |

## 6. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Local DDrawCompat selected runtime resource format | verified | `DDrawCompat-gamemd.log:166-168` | none for logged local run |
| Directory 16-bit runtime config | verified | `DDrawCompat.ini:6-8`; log final config lines `28`, `43`, `58` | none |
| Gamemd descriptor-to-shift/loss derivation | verified | assembly around `0x004BA9D0..0x004BAA83` | no attached runtime memory read |
| RGB555 classifier | verified | `0x004BAB4E..0x004BAB7D` | none |
| RGB565 classifier | verified | `0x004BABBE..0x004BABD9` | none |
| Third classifier value | touched-not-exhausted | `0x004BAB99..0x004BAC07` | semantic name/use outside ordinary RGB555/RGB565 remains out of scope |
| Text packing consumer | verified | `FUN_00621040 @ 0x00621040` | none |
| Minimap generated terrain packing | verified | `RadarClass__GenerateTerrainSurface @ 0x006547C0` | none |
| Minimap object/fog packing | verified | `RadarClass__RenderCellPixel @ 0x00655C50` | none |
| AlphaBlendRect masks | verified | `FUN_0060F9A0`; `AlphaBlendRect @ 0x00621B80` | none for mask source; exact per-call rects are separate reports |
| Direct `gamemd.exe` in-memory globals | deferred | debugger server unavailable | attach debugger after launch and read `0x008A0DD0..0x008A0DE4` if future proof wants raw memory |

## 7. Open Questions - Final State

- `[RESOLVED] OQ-01 - Does the local installed DirectDraw runtime choose RGB555 or RGB565? -> RGB565 / R5G6B5 in the logged local run.` (evidence: `DDrawCompat-gamemd.log:166-168`; Active in YR: Conditional on local DDrawCompat launch)
- `[RESOLVED] OQ-02 - Does the local config force 16-bit resource/display depth? -> It advertises/uses 16-bit depth: `SupportedDepthFormats=D16`, `DesktopColorDepth=16`, `RenderColorDepth=app`.` (evidence: `DDrawCompat.ini:6-8`; `DDrawCompat-gamemd.log:28,43,58`; Active in YR: Conditional)
- `[RESOLVED] OQ-03 - Does gamemd hardcode RGB565? -> No; it derives shifts/losses from descriptor masks and classifies known layouts.` (evidence: `0x004BA9D0..0x004BAC27`; Active in YR: Yes)
- `[RESOLVED] OQ-04 - What RGB565 globals result from the binary classifier? -> RShift=11, RLoss=3, GShift=5, GLoss=2, BShift=0, BLoss=3, classifier=2.` (evidence: `0x004BABBE..0x004BABD9`; Active in YR: Yes for local runtime)
- `[RESOLVED] OQ-05 - What RGB555 globals result from the binary classifier? -> RShift=10, RLoss=3, GShift=5, GLoss=3, BShift=0, BLoss=3, classifier=0.` (evidence: `0x004BAB4E..0x004BAB7D`; Active in YR: Supported but not locally selected)
- `[RESOLVED] OQ-06 - Do minimap terrain pixels consume the same globals? -> Yes, generated terrain surface writes 16-bit packed pixels through `g_DD_*`.` (evidence: `RadarClass__GenerateTerrainSurface @ 0x006547C0`; Active in YR: Yes)
- `[RESOLVED] OQ-07 - Do object dots/fog consume the same globals? -> Yes, object colors pack via globals, fog unpacks/re-halves/re-packs via globals.` (evidence: `RadarClass__RenderCellPixel @ 0x00655C50`; Active in YR: Yes)
- `[RESOLVED] OQ-08 - Does sidebar text consume the same globals? -> Yes, `FUN_00621040` packs source RGB into display-format packed16 before BitFont drawing.` (evidence: `0x00621040`; Active in YR: Yes)
- `[RESOLVED] OQ-09 - Does AlphaBlendRect consume independent constants? -> No, its masks derive from the same runtime shifts/losses.` (evidence: `FUN_0060F9A0`, `0x00621B80`; Active in YR: Yes)
- `[DEFERRED] OQ-10 - Can we read live `gamemd.exe` globals directly after launch?` (category: `needs-runtime-debugger`; reason: debugger server was not running and no game process was attached; next-step-if-pursued: launch under debugger and read `0x008A0DD0..0x008A0DE4` after `DSurface__Constructor`)

## 8. Visual/UI Composition Ledger

This report covers pixel-format conversion, not full minimap/sidebar composition. The active visual consumers proved here are:

| Order | Function / address | Condition / flag proof | Asset / frame | Rect / anchor | Palette / convert | Active for target? | Role |
|---|---|---|---|---|---|---|---|
| format init | `DSurface__Constructor` region `0x004BA9D0..0x004BAC27` | DirectDraw surface creation after 16 bpp mode setup | none | primary surface descriptor | DirectDraw descriptor masks -> globals | yes | format contract |
| terrain pack | `RadarClass__GenerateTerrainSurface @ 0x006547C0` | ordinary radar generated-surface path | TMP/overlay raw RGB already resolved elsewhere | generated radar surface | RGB -> RGB565 packed16 on local runtime | yes | minimap content |
| dirty/object/fog pack | `RadarClass__RenderCellPixel @ 0x00655C50` | radar dirty/full refresh paths | object/terrain pixels | radar primary cell pixel | RGB/fog packed16 globals | yes | minimap content |
| sidebar text | `FUN_00621040 @ 0x00621040` | owner-draw/sidebar text calls | BitFont glyphs | caller rect | source RGB -> packed16 | yes | UI text |
| dark strip blend | `AlphaBlendRect @ 0x00621B80` | sidebar dark-strip callers | none | caller rect | packed mask integer blend | yes | UI overlay |

Asset role matrix: not applicable; no new asset roles were investigated.

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Local runtime selects RGB565 (`D3DDDIFMT_R5G6B5`) for primary/plain 16-bit resources, and gamemd's RGB565 branch yields `RShift=11 RLoss=3 GShift=5 GLoss=2 BShift=0 BLoss=3`. | Runtime log `DDrawCompat-gamemd.log:166-168`; classifier assembly `0x004BABBE..0x004BABD9`; Active in YR: Conditional local runtime. | Missing/mismatch: Rust minimap/sidebar paths mostly operate in RGBA textures and comments/tests sometimes assume RGB565 without descriptor provenance. | `src/render/minimap.rs`, `src/render/minimap_helpers.rs`, `src/render/bit_font.rs`, future native packed-color helper. | Add an explicit `DisplayPixelFormat::Rgb565` fixture for the local runtime and drive minimap/sidebar packed-color tests through it. | On local-runtime fixture, RGB `(255,255,255)`, `(255,0,0)`, `(0,255,0)`, `(0,0,255)`, `(255,255,0)` pack to `0xFFFF`, `0xF800`, `0x07E0`, `0x001F`, `0xFFE0`. Proposed test: `ddraw_local_runtime_rgb565_pack_matches_gamemd_shift_loss`. | Do not call RGB565 universal; it is local-runtime evidence plus binary-supported classifier. |
| Gamemd still supports RGB555 and third classifier paths, so tests should parameterize the binary formula even though the local runtime is RGB565. | RGB555 branch `0x004BAB4E..0x004BAB7D`; third classifier `0x004BAB99..0x004BAC07`; Active in YR: supported, locally not selected. | Partial: Rust has RGB565-specific missing-glyph comments/math in `src/render/bit_font.rs`. | `src/render/bit_font.rs`, shared display-format tests. | Keep RGB555/RGB565 fixture tests separate; choose RGB565 only for local install fixtures. | Same source RGB yellow produces `0xFFE0` in RGB565 and `0x7FE0` in RGB555; both display yellow but packed bytes differ. Proposed test: `ddraw_rgb555_and_rgb565_fixtures_do_not_share_packed_values`. | Do not delete RGB555 support just because the local wrapper selected RGB565. |
| Minimap terrain/object/fog pixels consume global shift/loss values before sidebar blit. | `RadarClass__GenerateTerrainSurface @ 0x006547C0`; `RadarClass__RenderCellPixel @ 0x00655C50`; Active in YR: Yes. | Mismatch: `src/render/minimap.rs` stores/reuploads RGBA scratch texture and `minimap_helpers.rs` dims colors in RGBA. | `src/render/minimap.rs`, `src/render/minimap_helpers.rs`. | Reproduce native packed16 intermediate behavior for parity-critical minimap pixels, then expand for GPU display only after exact packed result exists. | Fogged terrain pixel should unpack RGB565 through shifts/losses, halve 8-bit-like expanded channels with native truncation, then repack exactly. Proposed test: `minimap_fog_half_bright_rgb565_unpacks_halves_repacks_like_gamemd`. | Do not implement fog as RGBA multiply or alpha overlay. |
| AlphaBlendRect uses derived packed masks and `>> 8` integer blend, not normalized RGBA alpha. | `FUN_0060F9A0`; `AlphaBlendRect @ 0x00621B80`; Active in YR: Yes. | Mismatch: `src/render/bit_font.rs` creates a 1x1 RGBA darken texture with `DARKEN_ALPHA`. | `src/render/bit_font.rs`, retained sidebar rasterizer. | For pixel parity, compute dark strips in native RGB565 packed space using derived masks. | Black source with alpha `0xAF` keeps `0x50/0x100` of each masked destination component, with truncation. Proposed test: `sidebar_dark_strip_rgb565_alpha_af_uses_packed_mask_shift8_math`. | Do not use GPU alpha blend and assume equality. |

### Applied Corrections / Follow-up Scope

- `docs/research/BITFONT_SHELL_TEXT_GHIDRA_REPORT.md` now records native 16-bit packing as a final-pixel constraint for the enrolled presentation:

  > For final pixel parity, the 16-bit DirectDraw packing is a parity constraint for native sidebar/minimap/text surfaces. The local DDrawCompat runtime log selects `D3DDDIFMT_R5G6B5`, and the sealed enrolled AMD/DDrawCompat/DXGI guard constrains its observed final-byte expansion. This remains environment-scoped evidence.

- `docs/research/PIXEL_FX_SPARKLES_GHIDRA_REPORT.md` now records the local format as resolved while preserving the runtime-derived general rule:

  > For the local DDrawCompat-backed install, runtime log evidence selects `D3DDDIFMT_R5G6B5` for primary/plain 16-bit resources. Gamemd derives RGB565 shift/loss globals from the primary surface descriptor if the descriptor masks match R5G6B5. Other installs still need descriptor sampling or wrapper logs before claiming RGB565.

- Any wording that says "the game uses RGB565" without scope should be replaced with:

  > Gamemd requests 16 bpp and derives display-format shifts/losses from the DirectDraw primary surface descriptor. The local DDrawCompat runtime selected `D3DDDIFMT_R5G6B5` in the recorded run, so local screenshot-parity fixtures should use RGB565; RGB555 remains binary-supported for other runtimes.

## 10. Negative Facts / Do Not Do

- Do not hardcode RGB555. Evidence: local DDrawCompat selected `D3DDDIFMT_R5G6B5` for primary/plain resources; Active in YR: Conditional local runtime.
- Do not state RGB565 is universal gamemd behavior. Evidence: gamemd has descriptor-derived shift/loss and explicit RGB555 classifier; Active in YR: Yes.
- Do not treat `DAT_008205D0` as a channel mask. Evidence: getter `0x004BBC90` returns classifier; actual packers read `0x008A0DD0..0x008A0DE4`; Active in YR: Yes.
- Do not keep minimap/sidebar parity judgments in RGBA space only. Evidence: `0x006547C0`, `0x00655C50`, and `0x00621040` write/read native packed16 pixels; Active in YR: Yes.
- Do not use floating or `/255` alpha for `AlphaBlendRect` parity. Evidence: `0x00621B80` uses packed masks, integer multiply, and `>> 8`; Active in YR: Yes.

## 11. Remaining Uncertainty

- Direct `gamemd.exe` memory read of `0x008A0DD0..0x008A0DE4` was not possible because the debugger server was not running. The DDrawCompat log is still a runtime sample of the local DirectDraw wrapper's resource-format selection.
- The current log identifies the process as `gamemd.exe`; it is still wrapper
  runtime evidence rather than an attached in-process memory read. A future
  debugger capture can confirm the exact post-constructor globals inside
  `gamemd.exe`.
- Third classifier semantic name remains out of scope. Preserve it as a fixture possibility; do not implement only two branches in a way that prevents later support.
- Final GPU screenshot/capture color-space equality remains an implementation-verification problem after native packed16 values are reproduced.

## Sources

- Ghidra read-only assembly/decompile: `DSurface__Constructor` region `0x004BA9D0..0x004BAC27`
- Ghidra read-only assembly: getters `0x004BBC30`, `0x004BBC40`, `0x004BBC50`, `0x004BBC60`, `0x004BBC70`, `0x004BBC80`, `0x004BBC90`
- Ghidra read-only decompile: `FUN_00621040 @ 0x00621040`
- Ghidra read-only decompile: `RadarClass__GenerateTerrainSurface @ 0x006547C0`
- Ghidra read-only decompile: `RadarClass__RenderCellPixel @ 0x00655C50`
- Ghidra read-only decompile: `RadarClass__RebuildRadarSurfaces @ 0x00654650`
- Ghidra read-only decompile: `FUN_0060F9A0`; `AlphaBlendRect @ 0x00621B80`
- Runtime/config files: `<ra2-install>/DDrawCompat.ini`; `<ra2-install>/DDrawCompat-gamemd.log`
- Prior docs: `docs/research/DIRECTDRAW_RUNTIME_PIXEL_FORMAT_MASKS_GHIDRA_REPORT.md`; `docs/research/ALPHABLENDRECT_0xAF_DARK_STRIP_PIXEL_MATH_GHIDRA_REPORT.md`; `docs/research/FUN_00621040_RGB_BYTE_PERMUTATION_GHIDRA_REPORT.md`; `docs/research/MINIMAP_GENERATED_PIXEL_COLOR_PIPELINE_GHIDRA_REPORT.md`

## Status

COMPLETE for the local DDrawCompat-backed runtime pixel-format sample and Rust-facing minimap/sidebar pixel-format contract: the runtime log selects RGB565 (`D3DDDIFMT_R5G6B5`), and gamemd's active binary consumers route minimap/sidebar/text/alpha pixels through the descriptor-derived shift/loss globals.

Remaining caveat: no attached debugger memory read was available, so this does not include a raw in-process dump of `gamemd.exe` globals after `DSurface__Constructor`.
