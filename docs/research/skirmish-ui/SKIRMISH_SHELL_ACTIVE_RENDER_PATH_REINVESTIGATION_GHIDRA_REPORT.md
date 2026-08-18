# Skirmish Shell Active Render Path Reinvestigation - Ghidra Evidence Synthesis

**Address(es):** `0x006AE3F0`, `0x006AE6E0`, `0x006ACEE0`, `0x00622B50`, `0x0060F9A0`, `0x00612B70`, `0x006153E0`, `0x00617250`, `0x00640710`, `0x00640A40`, `0x0072CF40`
**Confidence:** Medium-high overall; high where cited prior Ghidra reports already decompiled the path; low for background asset composition that still lacks a fresh live Ghidra trace.
**Active in YR:** Yes for offline Skirmish dialog `0x102`, owner-draw setup, Skirmish buttons, flag statics, color combos, and map preview start-marker drawing. Conditional/unresolved for some broad shell background candidates.

## 1. Overview

This pass rechecks the active Yuri's Revenge offline Skirmish setup shell after the first Rust shell replacement mixed verified Skirmish behavior with generic shell/sidebar assumptions.

Live Ghidra was not available in this session: `list_instances` returned no running instances. Therefore this report is a synthesis of already-verified Ghidra reports plus current Rust inspection, not a fresh binary decompilation. Any item that would require a new trace is explicitly marked as open.

The main correction is that Skirmish dialog `0x102` is not a single backdrop plus arbitrary sprites. It is a fullscreen-hosted Win32 dialog resource whose child controls are repositioned by shell helpers and then painted by the common owner-draw framework. Some right-panel assets are active and verified; several background/menu assets are only broad shell candidates and should not be treated as offline Skirmish evidence without a direct draw trace.

## 2. Verified Active YR Path

| Finding | Evidence | Confidence | Active in YR? |
|---|---|---:|---|
| Offline Skirmish uses dialog resource `0x102` and procedure `0x006AE3F0`. | `SKIRMISH_SHELL_VIEWPORT_ORIGIN_GHIDRA_REPORT.md`, `SKIRMISH_SHELL_LAYOUT_ASSETS_GHIDRA_REPORT.md`. | High | Yes |
| Dialog `0x102` is moved to shell origin `(0,0)` and covers the screen. | `FUN_0060C4A0`/common shell host findings in viewport-origin report. | High | Yes |
| Common shell init enumerates child controls, installs owner-draw hooks, and sets parent background state. | `FUN_00622B50`, `FUN_0060F9A0`, `FUN_0060CF00` in viewport/owner-draw reports. | High | Yes |
| `0x617` Start, `0x5AA` Choose Map, and `0x468` preview are right-anchored through `FUN_0060B1D0`; color combos and flag statics are not right-anchored. | Viewport-origin reports. | High | Yes |
| Back button `0x5C0` is positioned by the bottom/right-panel helper using `SDBTNANM.SHP` dimensions and right-panel globals. | Follow-up viewport report. | High | Yes |
| Main Skirmish buttons use `bue_*30.pcx` unpressed and `bde_*30.pcx` pressed; `bud_*` is not used by the normal offline Skirmish button path. | Owner-draw callbacks follow-up. | High | Yes |
| Owner-draw PCX controls use embedded PCX palettes decoded from the PCX files; no direct xrefs from that PCX conversion path to `DIALOG.PAL`, `SHELL.PAL`, or `MAINBTTN.PAL` were found in the prior report. | Owner-draw callbacks follow-up, section 1.4. | High for PCX path; medium for all surfaces | Yes |
| Flag statics `0x6DA..0x6E1` receive PCX surfaces through the side-combo helper path; missing flag PCX can render blank. | Owner-draw callbacks report, static callback/flag flow. | High | Yes |
| Map preview start markers use `STARTBUT.SHP` frame `0` and text label `i + 1` inside final child `0x468`. | `DrawStartPositions @ 0x00640710` in retail-assets and layout reports. | High | Yes |
| `mmpb.shp` is a separate map-preview/player-marker asset used by `FUN_00640A40`, not a generic map-preview backdrop. | Retail-assets and owner-draw asset mapping reports. | High for function use; medium for exact offline-screen moment | Yes, but not as a placeholder backdrop |

## 3. TS-Legacy / Generic-Shell Risk Register

| Asset / behavior | Status | Why it matters |
|---|---|---|
| `SDTP.SHP`, `SDBTNBKGD.SHP`, `SDBTM.SHP`, `SDBTNANM.SHP` | Active right-panel/sidebar-shell assets used by the shell path; not identified as TS-only. | These are defensible for layout/back-button/right-panel work. |
| `MNSCRNL.SHP`, `MNSCRNS.SHP` | Real assets with dimensions documented in the follow-up report, but current evidence in the prior reports does not prove they are the whole offline Skirmish background composition. | They can be loaded for research, but treating them as "the Skirmish background" is stronger than the evidence supports. |
| `MnScrnLCustomizeBattle.shp/.PAL` | Real broad shell/WOL-style screen asset; explicitly not proven as offline Skirmish background. | Do not use as a Skirmish `0x102` background target without direct xref or screenshot proof. |
| `dbak6440.pcx`, `dlgsysa.pcx`, `dlgsysi.pcx` | Verified owner-draw/background-class PCX pool assets, not all proven visible on offline Skirmish first viewport. | Better candidates for dialog-system chrome than arbitrary SHP backgrounds, but still need composition trace. |
| `sidebar.pal` | Not supported for owner-draw PCX buttons/flags; prior owner-draw reports point to embedded PCX palettes for those controls. | Rust's current `sidebar.pal` fallback is a wrong assumption for shell PCX controls. |
| `STARTBUT.SHP` | Verified numbered start marker. | Must be used for available start positions; not interchangeable with `mmpb.shp`. |
| `mmpb.shp` | Verified smaller player/house marker in `FUN_00640A40`. | Rust currently using it as a preview placeholder is not faithful. |
| hardcoded country-to-flag mapping by enum family | Wrong level of evidence. The binary maps side/item data to exact PCX names, including `japi.pcx`, `frai.pcx`, `geri.pcx`, `gbri.pcx`, `djbi.pcx`, `arbi.pcx`, `lati.pcx`, `rani.pcx`, `obsi.pcx`. | The Rust implementation collapses multiple countries to `usai.pcx` or `rusi.pcx`, losing visible country-specific icons. |
| sidebar font/text path | Not verified for shell owner-draw. Owner-draw text goes through `FUN_00621040` and bitfont clipping/color conversion. | Rust sidebar text may be visually wrong even if the labels are correct. |

## 4. Current Rust Implementation Status

The current Rust shell implementation has a good split of responsibilities but several evidence mismatches.

### 4.1 Defensible pieces

| Rust file | Status |
|---|---|
| `src/ui/skirmish_shell/layout.rs` | Mostly aligned with verified dialog/right-panel geometry: right-anchor controls, color/flag non-transform behavior, right-panel tile globals, and Back button `156x42` placement. |
| `src/assets/pcx_file.rs` | The parser direction is correct for owner-draw PCX assets because verified PCXs are 8-bit, one-plane, RLE, embedded-palette files. It still needs comparison against gamemd conversion edge cases. |
| `src/render/skirmish_shell_chrome.rs` loading `bue_*30.pcx`/`bde_*30.pcx` | Asset family is correct for normal Skirmish Start/Choose/Back buttons. |

### 4.2 Mismatches that should be fixed before another visible replacement

| Rust file / line | Current behavior | Evidence problem |
|---|---|---|
| `src/render/skirmish_shell_chrome.rs:57` | Uses `sidebar.pal` before `SHELL.PAL`/`DIALOG.PAL` for SHP rendering. | Owner-draw PCX controls use embedded PCX palettes; sidebar palette is not evidence for Skirmish shell controls. |
| `src/render/skirmish_shell_chrome.rs:77` | Loads `MNSCRNL.SHP`, `MNSCRNS.SHP`, `STARTBUT.SHP`, and `mmpb.shp` as the same optional bucket. | `STARTBUT.SHP` and `mmpb.shp` are map-marker assets; `MNSCRN*` are background candidates, not proven equivalent surfaces. |
| `src/app_skirmish_shell_render.rs:201` | Draws `mmpb.shp` or `SDMPBTN.SHP` fitted into map preview. | Verified preview path is child `0x468`, scenario preview draw helper, then `STARTBUT.SHP` markers; `mmpb.shp` is not a generic preview backing. |
| `src/app_skirmish_shell_render.rs:136` | Hardcodes country family to one of `usai.pcx`, `rusi.pcx`, `yrii.pcx`, `obsi.pcx`. | Ghidra reports verify a fuller item-data to PCX mapping. |
| `src/app_skirmish_shell_render.rs:242` | Uses `sidebar_text` for shell labels. | Owner-draw text wrapper and shell bitfont path remain to be matched. |
| `src/app_skirmish_shell_render.rs:89` | Composes button PCX pieces approximately. | Correct family, but needs exact cap/middle tiling and text vertical placement from `OwnerDraw_Button_00612B70`/`FUN_006BA3E0`. |

## 5. Implementation Guidance From This Recheck

1. Do not treat `MNSCRNL.SHP`, `MNSCRNS.SHP`, or `MnScrnLCustomizeBattle.shp` as the Skirmish background until a live Ghidra trace or screenshot comparison proves the composition.
2. Keep the right-panel geometry and active `SDBTN*` dimensions; those are supported by the viewport follow-up report.
3. For PCX owner-draw controls, decode embedded PCX palettes and stop using `sidebar.pal` as the first-choice shell palette.
4. Implement flags from the binary item-data to PCX mapping, not from Rust country families.
5. Treat the preview as a dedicated `0x468` rendering problem: map thumbnail first, then `STARTBUT.SHP` available-start markers, and only then investigate `mmpb.shp` for assigned-player markers.
6. If the shell is enabled before this is complete, gate unverified art behind a development flag or leave it blank with a log. Do not silently substitute plausible shell assets.

## 6. Open Questions

1. Fresh Ghidra trace needed: exact `FUN_0072CF40` Skirmish background/palette resource names and how `DAT_00B0FCDC` / `DAT_00B0FCE0` are consumed by shell paint.
2. Fresh Ghidra trace needed: exact parent background painting order in `FUN_00622B50` / common shell paint path for dialog `0x102`.
3. Fresh Ghidra trace needed: exact `OwnerDraw_Button_00612B70` cap/middle tiling edge behavior and text y offset.
4. Fresh Ghidra trace needed: exact font asset/bitfont identity used by `FUN_00621040` for these labels.
5. Live screenshot still needed at 800x600 and 1024x768 to validate the final background/right-panel composition.

## Sources

- `C:/Users/enok/Documents/ra2-rust-game-docs/SKIRMISH_SHELL_LAYOUT_ASSETS_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/SKIRMISH_SHELL_RETAIL_ASSETS_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/SKIRMISH_OWNERDRAW_ASSET_MAPPING_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/SKIRMISH_SHELL_VIEWPORT_ORIGIN_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/SKIRMISH_SHELL_VIEWPORT_ORIGIN_FOLLOWUP_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/SKIRMISH_OWNERDRAW_CALLBACKS_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/SKIRMISH_OWNERDRAW_CALLBACKS_FOLLOWUP_GHIDRA_REPORT.md`
- Current Rust scan: `src/ui/skirmish_shell/layout.rs`, `src/render/skirmish_shell_chrome.rs`, `src/app_skirmish_shell_render.rs`

