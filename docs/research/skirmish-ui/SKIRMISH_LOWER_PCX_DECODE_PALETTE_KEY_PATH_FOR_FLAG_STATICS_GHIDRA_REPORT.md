# Skirmish Lower PCX Decode Palette Key Path For Flag Statics - Ghidra Research Report

**Address(es):** `0x006B9D00` (owner-draw PCX cache loader), `0x00630310` (PCX file/surface decoder), `0x006BA140` (cache lookup), `0x006BA580` (keyed 16-bit blit), `0x006153E0` (static paint), `0x00612B70` (button paint), `0x006BA3E0` (button middle-piece tile blit), `0x0061F210` (preload pool)  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** Lower PCX decode, palette conversion, transparency keying, and native-size/keyed draw behavior as used by standard offline Skirmish dialog `0x102` flag statics and owner-draw PCX button assets.  
**Non-Scope:** Sidebar PCX/SHP palette behavior, generic SHP palette remap, online lobby-only PCXs, malformed PCX compatibility outside the retail owner-draw assets, and all non-button/non-flag owner-draw callbacks.  
**Confidence:** High for standard YR owner-draw PCX palette/key/draw path; Medium for generic PCX decoder edge rejection because this report did not exhaust malformed/non-retail files.  
**Active in YR:** Yes. The path is reached by the standard shell owner-draw setup and Skirmish dialog `0x102`; no TS-only gate was found in the scoped calls.

## 1. Overview

Standard offline Skirmish flag statics and PCX owner-draw buttons consume cached owner-draw PCX surfaces. The loader decodes retail PCX files with their embedded 256-entry VGA palette, converts indexed pixels into active 16-bit DirectDraw-format pixels, and caches that converted surface. Flag transparency is not an 8-bit palette-index rule; the static paint callback computes the display-format value for RGB magenta (`255,0,255`) and the keyed blitter skips source pixels equal to that converted 16-bit value.

Active in YR: Yes. Evidence: `FUN_0060F9A0` runs from the shell owner-draw setup, calls `FUN_0061F210` once, Skirmish flag selection uses `FUN_006BA140`, and `OwnerDraw_Static_006153E0 @ 0x006153E0` uses `FUN_006BA580` for kind-2 static images.

## 2. Class Layout / Key Offsets

| Object / record | Offset | Purpose | Active in YR |
|---|---:|---|---|
| Cached owner-draw PCX entry | `+0x04` | converted surface pointer copied out by `FUN_006BA140` | Yes; cache lookup returns this value |
| Cache entry next link | `+0x308` | hash-chain next pointer | Yes; lookup and insert traverse it |
| `BSurface`/`XSurface`-style surface | vtable `+0x5C` | lock/pixel pointer accessor | Yes; loader, keyed blit, and button blits call it |
| `BSurface`/`XSurface`-style surface | vtable `+0x60` | unlock/release after pixel access | Yes |
| `BSurface`/`XSurface`-style surface | vtable `+0x78` | surface rect metadata | Yes |
| `BSurface`/`XSurface`-style surface | vtable `+0x7C` | source width | Yes; static kind-2 reads before centering/clipping |
| `BSurface`/`XSurface`-style surface | vtable `+0x80` | source height | Yes; static kind-2 reads before centering/clipping |
| Static owner-draw state | logical kind `2` at prior docs' kind field | PCX/image static mode | Yes; flag statics use this mode |
| Static owner-draw state | image pointer field consumed as `piVar11[5]` in decompile | cached PCX surface for kind `2` | Yes |
| DirectDraw globals | `g_DD_RLoss/RShift`, `g_DD_GLoss/GShift`, `g_DD_BLoss/BShift` | convert RGB bytes to active 16-bit display format | Yes |

## 3. Core Logic

### 3.1 Decode and conversion

Active in YR: Yes. Evidence: `FUN_0061F210 @ 0x0061F210` preloads all scoped flag and button PCXs by calling the owner-draw cache loader with mode `2`, and `OwnerDraw_Button_00612B70 @ 0x00612B70` and `OwnerDraw_Static_006153E0 @ 0x006153E0` consume the resulting cached surfaces.

The lower path is:

1. `FUN_0061F210` preloads owner-draw PCXs, including `bue_li30.pcx`, `bue_mi30.pcx`, `bue_ri30.pcx`, `bde_li30.pcx`, `bde_mi30.pcx`, `bde_ri30.pcx`, and flag PCXs `rani/usai/japi/frai/geri/gbri/djbi/arbi/lati/rusi/obsi/yrii.pcx`.
2. Loader `0x006B9D00` zeroes a 256 RGB-triplet palette scratch area before decoding.
3. `BSurface__Constructor @ 0x00630310` validates and RLE-decodes the PCX raster into an indexed `BSurface`-style object. The decoder reads the final 768-byte palette block for 8-bit, one-plane retail PCX data.
4. For loader mode `2`, `0x006B9D00` converts the 256 RGB triplets into a 256-entry 16-bit table using DirectDraw loss/shift globals.
5. It allocates a destination 16-bit surface, locks the decoded indexed pixels through vtable `+0x5C`, and writes one 16-bit destination pixel per source index by table lookup.
6. The converted surface is inserted into the owner-draw cache. `FUN_006BA140 @ 0x006BA140` later returns the cached converted surface pointer or `0`.

Tiny details:

- The conversion uses the active display format at load time, not an RGBA buffer and not a palette file lookup. Active in YR: Yes; evidence `0x006B9D00` mode-2 conversion uses `g_DD_*Loss/*Shift`.
- The cache lookup copies the matching cache entry to scratch globals before returning the surface pointer. Active in YR: Yes; evidence `FUN_006BA140` copies `0xC1` dwords from entry `+0x04`.
- Missing decode/cache entry returns `0`. Active in YR: Yes; evidence `FUN_006BA140` returns `0` after failed chain search; prior flag-static path stores/checks null and paints background only.

### 3.2 Palette source

Active in YR: Yes. Evidence: `FUN_0061F210`, loader `0x006B9D00`, and `BSurface__Constructor @ 0x00630310`.

The owner-draw PCX path uses the PCX's embedded palette block. No scoped function loads or applies `SHELL.PAL`, `SHELL2.PAL`, `SIDEBAR.PAL`, `DIALOG.PAL`, or `MAINBTTN.PAL` to flag/button PCX pixels. Those palettes remain valid for separate SHP/right-panel surfaces, but they are not the palette source for owner-draw PCX flag/button assets.

Active in YR: Yes. Evidence: `batch_string_anchor_report("pcx")` maps flag/button strings to `FUN_0061F210`, `FUN_004E3560`, and `OwnerDraw_Button_00612B70`; palette strings are absent from the scoped PCX loader/lookup/blit functions. Prior asset report confirms checked flag PCXs carry the embedded VGA palette marker and 768-byte palette block.

### 3.3 Transparency key

Active in YR: Yes. Evidence: `OwnerDraw_Static_006153E0 @ 0x006153E0` computes the key from RGB magenta through `g_DD_*Loss/*Shift`; `FUN_006BA580 @ 0x006BA580` skips source pixels equal to the passed 16-bit key.

Key behavior:

- The static callback does not compare source index `0`, index `255`, or any PCX palette index.
- The keyed blit sees already-converted 16-bit source pixels.
- Any source palette entry that converts to the same 16-bit value as RGB magenta will be transparent in this blitter.
- Non-magenta index `0` pixels are not inherently transparent.

### 3.4 Native-size static draw and clipping

Active in YR: Yes. Evidence: `OwnerDraw_Static_006153E0 @ 0x006153E0`, kind-2 branch.

Kind-2 static paint restores the cached background first, then reads source width/height. If the source is smaller than the static rect in a dimension, it centers with integer division and uses source size. If the source is greater than or equal to the rect in a dimension, it keeps the destination extent at the control rect size, causing clipping rather than scaling. This confirms prior flag-static reports and is the draw rule Rust should use for flag PCXs.

### 3.5 Owner-draw buttons share the same PCX cache

Active in YR: Yes. Evidence: `OwnerDraw_Button_00612B70 @ 0x00612B70` formats `b%c%c_li%d.pcx`, `b%c%c_mi%d.pcx`, and `b%c%c_ri%d.pcx`, calls `FUN_006BA140`, then draws the left/right caps and tiles the middle through `FUN_006BA3E0`.

The normal Skirmish buttons select the `30` family at the verified 37-pixel client height. Their PCX pixels are already embedded-palette converted by the same loader path as the flags. The button path's middle-piece helper is not keyed by magenta; it copies/tile-fills 16-bit pixels.

## 4. INI Keys

No INI keys control this scoped PCX palette/key path. The standard Skirmish country order in `ini/rulesmd.ini [Countries]` explains side item-data order, but the PCX palette source, RGB magenta key, and owner-draw cache behavior are binary/asset behavior, not INI-configured behavior.

| Key | Status | Effect |
|---|---|---|
| `[Countries]` order in `rulesmd.ini` | contextual only | side item data maps to hardcoded flag PCX names in prior reports |
| PCX palette key | none found | no scoped INI key gates or overrides embedded palette conversion |

Active in YR: Yes for the absence in this scoped path. Evidence: no INI read or palette-key read in `FUN_0061F210`, `0x006B9D00`, `FUN_006BA140`, `OwnerDraw_Static_006153E0`, or `OwnerDraw_Button_00612B70`.

## 5. Integration Points

| Integration point | Status | Evidence | Active in YR |
|---|---|---|---|
| One-time owner-draw preload | verified | `FUN_0060F9A0` calls `FUN_0061F210` when preload guard is clear | Yes |
| Flag selection | verified by prior docs; checked here for cache contract | `FUN_004E3560` calls `FUN_006BA140(name, 0)` | Yes |
| Flag paint | verified | `OwnerDraw_Static_006153E0` kind-2 branch -> `FUN_006BA580` | Yes |
| Button paint | verified | `OwnerDraw_Button_00612B70` -> `FUN_006BA140` -> cap blits / `FUN_006BA3E0` | Yes |
| SHP right-panel palettes | verified separate path | `SKIRMISH_SHELL_RIGHT_PANEL_BACKGROUND_PALETTE_FOLLOWUP_GHIDRA_REPORT.md` | Yes, but separate from PCX controls |

## 6. Current Rust Implementation Status

Rust already has a real embedded-palette PCX parser in `src/assets/pcx_file.rs:17..100`, and `src/render/skirmish_shell_chrome.rs:158..178` loads the verified flag/button PCX filenames into the Skirmish chrome atlas.

Rust deltas:

- `src/render/skirmish_shell_chrome.rs:178` passes `Some(0)` as the transparent index for all scoped PCX entries. This is wrong for binary parity: gamemd uses RGB magenta converted to display format, not palette index `0`.
- `src/assets/pcx_file.rs:88..99` supports only index-based alpha conversion today. It needs a color-key path or equivalent atlas-time rule for magenta after palette lookup/conversion.
- `src/app_skirmish_shell_render.rs:607` and `:617` draw flags through `push_entry_fit`, which scales. The binary static path uses native-size centered/clipped draw.
- Button assets in `src/app_skirmish_shell_render.rs` are in the correct `bue_*30`/`bde_*30` family, but any future alpha/key shortcut should not apply flag magenta keying to button middle/cap PCXs unless a button-specific keyed call is proven.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Owner-draw PCX preload list | verified | `FUN_0061F210 @ 0x0061F210`; strings `0x00835DE4..0x00835E34`, `0x00836328..0x008363AC` | none for scoped assets |
| PCX file decode reads embedded palette | verified for retail 8-bit path | `BSurface__Constructor @ 0x00630310`; prior asset marker evidence | malformed PCX rejection outside scope |
| Mode-2 indexed-to-16-bit conversion | verified | loader `0x006B9D00`, DirectDraw loss/shift globals | none |
| Cache lookup returns converted surface | verified | `FUN_006BA140 @ 0x006BA140` | none |
| Static RGB-magenta key | verified | `OwnerDraw_Static_006153E0 @ 0x006153E0`; `FUN_006BA580 @ 0x006BA580` | none |
| Flag native-size clipping | verified by prior docs and rechecked | `OwnerDraw_Static_006153E0 @ 0x006153E0` | none |
| Button PCX cache use | verified | `OwnerDraw_Button_00612B70 @ 0x00612B70`; `FUN_006BA3E0 @ 0x006BA3E0` | exact text y/disabled overlay belongs to button-specific reports |
| External shell/sidebar palette use for scoped PCXs | verified negative | no scoped palette-file read in loader/lookup/static/button PCX functions | none |
| Rust parser and render mismatch | verified scan | `src/assets/pcx_file.rs:88..99`; `src/render/skirmish_shell_chrome.rs:178`; `src/app_skirmish_shell_render.rs:607,:617` | implementation only |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-LPCX-001 - Is the lower PCX loader live for standard YR Skirmish flags? -> Yes; flag PCXs are preloaded by `FUN_0061F210` and selected by `FUN_004E3560`/`FUN_006BA140`.` (evidence: `0x0061F210`, `0x004E3560`, `0x006BA140`)
- `[RESOLVED] OQ-LPCX-002 - Is the owner-draw PCX palette embedded or supplied by shell/sidebar PAL files? -> Embedded PCX palette for scoped PCX controls; no scoped external PAL read was found.` (evidence: `0x00630310`, `0x006B9D00`, string-anchor report)
- `[RESOLVED] OQ-LPCX-003 - Does static transparency key by index or color? -> Color; converted RGB magenta 16-bit value is passed to the keyed blitter.` (evidence: `0x006153E0`, `0x006BA580`)
- `[RESOLVED] OQ-LPCX-004 - Do flag statics scale the PCX? -> No; they center only when smaller and clip otherwise.` (evidence: `0x006153E0`; prior flag reports)
- `[RESOLVED] OQ-LPCX-005 - Do buttons use the same PCX converted cache? -> Yes; `OwnerDraw_Button_00612B70` formats PCX names and calls `FUN_006BA140`.` (evidence: `0x00612B70`)
- `[RESOLVED] OQ-LPCX-006 - Is `SIDEBAR.PAL` a valid shortcut for these PCX controls? -> No; sidebar palette is unrelated to the scoped owner-draw PCX conversion path.` (evidence: `0x006B9D00`, `0x0061F210`, `0x00612B70`, `0x006153E0`)
- `[RESOLVED] OQ-LPCX-007 - What should Rust change? -> Replace index-0 transparency with RGB-magenta color key for flag/static PCXs and draw flags native clipped, not fit-scaled.` (evidence: Rust scan lines cited in section 6; binary `0x006153E0`)
- `[DEFERRED] OQ-LPCX-008 - Exact behavior for malformed/non-retail PCX headers and unusual multi-plane variants.` (category: out-of-scope; reason: standard Skirmish flag/button PCXs are retail 8-bit one-plane assets; next-step-if-pursued: generic PCX compatibility investigation against `0x00630310`)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Owner-draw PCX controls use embedded PCX palette data, then convert indexed pixels; scoped path does not use shell/sidebar PAL files. | `0x00630310`, `0x006B9D00`; asset doc embedded palette marker; no scoped palette-string use | mostly aligned for palette parsing; ensure no external PAL fallback for PCX controls | `src/assets/pcx_file.rs`; `src/render/skirmish_shell_chrome.rs` | keep PCX-control colors from embedded palette, not `SHELL.PAL`/`SIDEBAR.PAL` | Loading `usai.pcx` and `bue_li30.pcx` produces colors from their embedded palettes without consulting shell/sidebar palettes; proposed test `skirmish_ownerdraw_pcx_uses_embedded_palette_not_shell_pal` | Do not route owner-draw PCXs through SHP palette selection |
| Flag static transparency is the converted RGB magenta key, not palette index 0. | `OwnerDraw_Static_006153E0 @ 0x006153E0`; `FUN_006BA580 @ 0x006BA580` | mismatch: `render_pcx_entry(..., Some(0))` at `src/render/skirmish_shell_chrome.rs:178`; `to_rgba` supports index only | `src/assets/pcx_file.rs`; `src/render/skirmish_shell_chrome.rs` | make flag/static PCX atlas alpha transparent when palette RGB converts to magenta key, and do not blank non-magenta index 0 pixels | Synthetic PCX with index 0 = black and index 7 = magenta should keep index 0 opaque and key index 7 transparent; proposed test `skirmish_flag_pcx_transparency_keys_magenta_not_index_zero` | Do not assume index `0` or `255` is transparent |
| Flag statics draw native size with centering only when smaller and clipping when larger. | `OwnerDraw_Static_006153E0 @ 0x006153E0`; prior flag reports | mismatch: flags use `push_entry_fit` at `src/app_skirmish_shell_render.rs:607` and `:617` | `src/app_skirmish_shell_render.rs` | emit flag sprite at native PCX size and clip to flag control rect; normal `47x23` in a `48x20` rect should clip vertically, not scale | At 800x600, a `47x23` flag in the verified `48x20` rect is drawn at 47 px width with vertical clipping to 20 px; proposed test `skirmish_flag_static_draws_native_size_and_clips_vertically` | Do not use fit-to-rect scaling for flags |

### Negative Facts / Do Not Do

- Do not apply `SHELL.PAL`, `SHELL2.PAL`, `SIDEBAR.PAL`, `DIALOG.PAL`, or `MAINBTTN.PAL` to flag/button PCX controls. Evidence: scoped loader/lookup/static/button path uses embedded PCX palette and DirectDraw conversion (`0x00630310`, `0x006B9D00`, `0x0061F210`, `0x00612B70`, `0x006153E0`). Active in YR: Yes.
- Do not implement flag transparency as palette index `0`. Evidence: static paint computes RGB magenta display key and `FUN_006BA580` compares converted 16-bit source pixels to that key. Active in YR: Yes.
- Do not implement flag transparency as palette index `255` either. Evidence: no index compare occurs in `OwnerDraw_Static_006153E0` or `FUN_006BA580`; the compare is against a 16-bit key value. Active in YR: Yes.
- Do not scale flag PCXs to the control rectangle. Evidence: kind-2 static branch centers when smaller and clips when larger. Active in YR: Yes.
- Do not infer `bud_*` disabled Skirmish button art from preload strings. Evidence: `OwnerDraw_Button_00612B70` formats normal paths as `bue_*`/`bde_*`, while disabled normal path forces unpressed base art and applies an alpha overlay; prior callback follow-up found `bud_*` preload-only in this path. Active in YR: Yes for the normal offline Skirmish buttons.

### Remaining Uncertainty

- Exact malformed/non-retail PCX rejection behavior in `BSurface__Constructor @ 0x00630310` remains out of scope. This does not affect standard offline Skirmish flag statics or the verified owner-draw button PCXs, which are retail 8-bit, one-plane, embedded-palette PCX assets.
- Runtime screenshot comparison could still be used to validate final Rust flag clipping after implementation, but the binary rule for native-size/clipped draw is resolved.

### Stale Docs / Follow-up Docs

- `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_SHELL_ACTIVE_RENDER_PATH_REINVESTIGATION_GHIDRA_REPORT.md` replacement for section 4.2 row about `src/render/skirmish_shell_chrome.rs:57`:
  - Replace: "`src/render/skirmish_shell_chrome.rs:57` uses `sidebar.pal` before `SHELL.PAL`/`DIALOG.PAL` for SHP rendering."
  - With: "`src/render/skirmish_shell_chrome.rs:73..178` now separates SHP palette paths from PCX loading; the remaining scoped PCX mismatch is that `render_pcx_entry(..., Some(0))` keys transparency by index 0, while gamemd's flag static path keys by converted RGB magenta. Owner-draw PCX controls should continue using embedded PCX palettes and must not be decoded through `SHELL.PAL`/`SIDEBAR.PAL`."

## Sources

- Ghidra read-only decompile: `0x00630310`, `0x006B9D00`, `0x006BA120`, `0x006BA140`, `0x006BA3E0`, `0x006BA580`, `0x0061F210`, `0x00612B70`, `0x006153E0`.
- Ghidra string-anchor report: PCX strings and xrefs including `bue_*30`, `bde_*30`, `rani/usai/japi/frai/geri/gbri/djbi/arbi/lati/rusi/obsi/yrii.pcx`.
- Prior docs: `SKIRMISH_OWNERDRAW_CALLBACKS_FOLLOWUP_GHIDRA_REPORT.md`, `SKIRMISH_FLAG_PCX_PALETTE_AND_NATIVE_CLIP_GHIDRA_REPORT.md`, `SKIRMISH_FLAG_STATIC_RENDERING_GHIDRA_REPORT.md`, `SKIRMISH_OWNERDRAW_ASSET_MAPPING_GHIDRA_REPORT.md`, `SKIRMISH_SHELL_RIGHT_PANEL_BACKGROUND_PALETTE_FOLLOWUP_GHIDRA_REPORT.md`.
- Rust scan only: `src/assets/pcx_file.rs:17..100`, `src/render/skirmish_shell_chrome.rs:158..178`, `src/render/skirmish_shell_chrome.rs:362..374`, `src/app_skirmish_shell_render.rs:123..170`, `src/app_skirmish_shell_render.rs:604..617`.
