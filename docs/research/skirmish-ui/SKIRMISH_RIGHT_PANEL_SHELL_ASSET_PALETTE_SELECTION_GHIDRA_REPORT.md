# Skirmish Right-Panel Shell Asset/Palette Selection - Ghidra Research Report

**Address(es):** `0x006AE2C0`, `0x006AE3F0`, `0x00622B50`, `0x00621E90`, `0x0060CF00`, `0x0072CF40`, `0x0072E450`, `0x0072E730`, `0x0072EC70`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** active parent/right-panel shell asset and palette selection for standard offline YR Skirmish dialog resource `0x102`.  
**Non-Scope:** combo controls, flags, owner-draw PCX buttons, map preview decode, child control text, and full WOL chrome semantics beyond the one flag needed to decide `SDBTNANM.SHP` visibility.  
**Confidence:** High for standard offline Skirmish `0x102` asset/palette selection; Medium-high for excluding `SDBTNANM.SHP` frame 10 from standard offline `0x102` first-paint right-panel chrome.  
**Active in YR:** Yes for the `0x102` path; `SDBTNANM.SHP` frame 10 is not active in standard offline `0x102`, but is conditional on WOL-family dialog record state elsewhere.

## 1. Overview

The standard offline Skirmish launcher reaches `FUN_006AE2C0`, calls `FUN_0072CF40`, then creates dialog resource `0x102` with proc `0x006AE3F0`. The common shell paint path draws the right-panel stack first, then the selected parent background overlay, then child controls.

For standard offline `0x102`, the active right-panel shell stack is `SDTP.SHP`, repeated `SDBTNBKGD.SHP`, `SDBTM.SHP`, and the width-selected lower strip `LWSCRNS.SHP`/`LWSCRNL.SHP`. `SDBTNANM.SHP` is loaded and has a verified palette binding, but its frame-10 right-panel row is not drawn on standard offline Skirmish unless the per-dialog WOL-family flag has been set; the only live setter call sites found are WOL dialog paths, not the offline Skirmish launcher.

## 2. Verified Findings

| Finding | Active in YR | Evidence | Confidence |
|---|---|---|---|
| Offline Skirmish launches dialog `0x102` after calling `FUN_0072CF40`. | Yes - standard offline Skirmish. | `FUN_006AE2C0`; assembly at `0x006AE317..0x006AE328`: call `0x0072CF40`, `EDX=0x006AE3F0`, `ECX=0x102`, call `0x00622650`. | High |
| `FUN_0072CF40` loads the 800-width Skirmish parent SHP only when `g_ScreenWidth == 800`, and always loads the Skirmish parent palette convert. | Yes - standard offline Skirmish entry. | `0x0072CF49` compares screen width to `0x320`; `0x0072CF55` uses string pointer table `0x00844D6C -> 0x00844FA8 = MnScrnLCoopGameSetup.shp`; `0x0072CF6A` uses `0x00844D70 -> 0x00844F8C = MnScrnLCoopGameSetup.PAL`; `0x0072CF75`/`0x0072CF70` pass raw palette `DAT_00B0FCDC` and convert `DAT_00B0FCE0`. | High |
| Dialog `0x102` stores `DAT_00B0FCE0`, `DAT_00B0FB50`, and `DAT_00B0FA18` as its parent convert/small-SHP/large-SHP fields. | Yes - standard offline Skirmish. | `FUN_0060CF00`, branch for `iVar3 == 0x102 || 0xBC || 0xBD || 0xC2 || 0xC9`: calls `FUN_0072D030` (`return DAT_00B0FCE0`), writes `piVar2[0x39]=DAT_00B0FB50`, `piVar2[0x3A]=DAT_00B0FA18`. | High |
| `DAT_00B0FB50` is `MNSCRNS.SHP`; `DAT_00B0FA04` is `MNSCRNL.SHP`. | Yes - `MNSCRNS.SHP` is the 640 small parent; `MNSCRNL.SHP` is not the standard 640 `0x102` parent. | `Sidebar_RightPanel_SHP_Loading @ 0x0072EB50` and `FUN_0072DFB0`: load table `0x00844CE0 -> 0x00845150 = MNSCRNS.SHP`, then store prior call result to `0x00B0FB50`; next table entry `0x00844CE4 -> 0x00845144 = MNSCRNL.SHP`, stored to `0x00B0FA04`. | High |
| `Background_Overlay` selects the small parent only at width `640`, otherwise the alternate parent pointer. | Yes - standard shell paint path. | `Background_Overlay @ 0x0072E730`: if `g_ScreenWidth == 0x280`, draws `param_4`; else draws `param_5`; frame is `0`, flags include `0x400`. | High |
| Null alternate parent SHP is a no-op. This matters for fresh non-640/non-800 entry because `DAT_00B0FA18` is only loaded at exact width `800` and cleanup clears it. | Conditional - relevant outside standard 640/800 shell sizes. | `FUN_0072CF40` width gate; `FUN_0072CF90` clears `DAT_00B0FA18`; `CC_Draw_Shape @ 0x004AED70` returns before frame work if the SHP pointer path is null. | High for static behavior; Medium for stale runtime history. |
| `SDTP.SHP` is drawn as the right-panel top cap, frame `0`, with the `SHELL.PAL` convert object. | Yes - standard offline `0x102` right-panel paint. | Load table `0x00844CE8 -> SDTP.SHP`, stored to `DAT_00B0FAF8`; `RightPanel__Draw @ 0x0072E547..0x0072E567` uses `DAT_00B0FAF8`, frame `0`, convert `DAT_00B0FBCC`; palette table `0x00844BE4 -> SHELL.PAL`. | High |
| `SDBTNBKGD.SHP` is drawn as the repeated right-panel tile, frame `0`, with the `SHELL2.PAL` convert object. | Yes - standard offline `0x102` right-panel paint. | Load table `0x00844CEC -> SDBTNBKGD.SHP`, stored to `DAT_00B0FA74`; `RightPanel__Draw @ 0x0072E594..0x0072E5C1` loops `DAT_00B0FA20` times, row stride `DAT_00B0FC24[3]`, convert `DAT_00B0FBD4`; palette table `0x00844BE8 -> SHELL2.PAL`. | High |
| `SDBTM.SHP` is drawn as the right-panel bottom cap, frame `0`, with the `SHELL.PAL` convert object. | Yes - standard offline `0x102` right-panel paint. | Load table `0x00844CF0 -> SDBTM.SHP`, stored to `DAT_00B0FA38`; `RightPanel__Draw @ 0x0072E68C..0x0072E69F` uses `DAT_00B0FA38`, frame `0`, convert `DAT_00B0FBCC`; palette table `0x00844BE4 -> SHELL.PAL`. | High |
| `SDBTNANM.SHP` is loaded and palette-bound, but its right-panel frame-10 overlay is not drawn for standard offline Skirmish `0x102`. | No for standard offline `0x102`; Conditional in YR WOL-family dialogs. | Load table `0x00844CD4 -> SDBTNANM.SHP`, stored to `DAT_00B0FAC4`; palette table `0x00844BEC -> SDBTNANM.PAL`, convert `DAT_00B0FBDC`; branch at `RightPanel__Draw @ 0x0072E5E2..0x0072E65C` skips when `param_3 != 0`; caller at `0x00621FEC..0x00621FFE` passes `param_3 = (record[+0xD8] == 0)`; `FUN_00623340` zero-fills new records; live xrefs to setter `FUN_00608440` are only `0x0078B808`, `0x0078BF87`, `0x00792DA6`, `0x00793407` WOL paths per xref check and prior setter-caller report. | Medium-high |
| `LWSCRNS.SHP`/`LWSCRNL.SHP` are the lower strip assets, selected by width, using `SHELL.PAL`. | Yes - lower strip is part of the same right-panel draw helper. | Load table `0x00844CF4 -> LWSCRNS.SHP`, stored to `DAT_00B0FAE8`; `0x00844CF8 -> LWSCRNL.SHP`, stored to `DAT_00B0FA54`; `RightPanel__Draw @ 0x0072E6BD..0x0072E71F` chooses `LWSCRNS` only at `g_ScreenWidth == 0x280`, else `LWSCRNL`; convert `DAT_00B0FBCC`. | High |

## 3. Palette Loader Details

`FUN_0072ADE0` is the palette-to-convert-object helper used here. It reads a 256-entry palette, shifts each 6-bit channel left by 2, stores the raw RGB palette at the `EDX` target, then constructs a convert object at the pushed target against `DAT_00887310`.

| Convert object | Raw palette | Source PAL | Active in YR | Evidence |
|---|---|---|---|---|
| `DAT_00B0FBCC` | `DAT_00B0FBC8` | `SHELL.PAL` | Yes - `SDTP`, `SDBTM`, `LWSCRNS/LWSCRNL`. | `0x0072E46C..0x0072E47C` and `0x0072E143..0x0072E158`; pointer table `0x00844BE4 -> 0x00845454 = SHELL.PAL`. |
| `DAT_00B0FBD4` | `DAT_00B0FBD0` | `SHELL2.PAL` | Yes - `SDBTNBKGD`. | `0x0072E481..0x0072E491` and `0x0072E15D..0x0072E16D`; pointer table `0x00844BE8 -> 0x00845448 = SHELL2.PAL`. |
| `DAT_00B0FBDC` | `DAT_00B0FBD8` | `SDBTNANM.PAL` | Conditional in YR; not visible in standard offline `0x102` right-panel first paint. | `0x0072E496..0x0072E4A6` and `0x0072E172..0x0072E182`; pointer table `0x00844BEC -> 0x00845438 = SDBTNANM.PAL`. |
| `DAT_00B0FCE0` | `DAT_00B0FCDC` | `MnScrnLCoopGameSetup.PAL` | Yes - parent background convert for dialog `0x102`. | `0x0072CF6A..0x0072CF7A`; `FUN_0072D030` returns `DAT_00B0FCE0`; xrefs to `DAT_00B0FCDC/FCE0` are `FUN_0072CF40`, `FUN_0072CF90`, and `FUN_0072D030`. |

## 4. Integration Points

Active standard offline path:

`FUN_006AE2C0 -> FUN_0072CF40 -> FUN_00622650(dialog 0x102, proc 0x006AE3F0) -> FUN_00622B50 -> WM_PAINT_Handler @ 0x00621E90 -> RightPanel__Draw @ 0x0072E450 -> Background_Overlay @ 0x0072E730`.

`RightPanel__Draw` runs before `Background_Overlay` in `WM_PAINT_Handler`. Child controls and map preview work are outside this report's scope.

## 5. Current Rust Implementation Status

Not scanned in this slot. The parent task constrained this subagent to one research report and no in-repo edits; Rust implementation comparison is intentionally deferred.

## 6. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Offline launcher to dialog `0x102` | verified | `0x006AE317..0x006AE328` | none |
| `FUN_0072CF40` parent SHP/PAL loader | verified | `0x0072CF40`, `0x00844D6C`, `0x00844D70`, `0x00844F8C`, `0x00844FA8` | none |
| `DAT_00B0FCDC/DAT_00B0FCE0` consumption | verified for this scope | xrefs to `0x00B0FCDC`, `0x00B0FCE0`; `FUN_0072D030` | no broader palette-system audit |
| `FUN_0060CF00` dialog `0x102` parent metadata | verified | `0x0060CF00` branch for ids `0x102/0xBC/0xBD/0xC2/0xC9` | none |
| `RightPanel__Draw` SHP order and palette split | verified | `0x0072E450`; draw sites `0x0072E547`, `0x0072E594`, `0x0072E60D`, `0x0072E68C`, `0x0072E6CD/0x0072E6F7` | none for listed assets |
| `SDBTNANM.SHP` standard offline visibility | verified-with-prior-crosscheck | `0x00621FEC..0x00621FFE`, `0x0072E5E2..0x0072E65C`, `FUN_00623340`, `FUN_00608440` xrefs; `SDBTNANM_FRAME10_SETTER_CALLERS_GHIDRA_REPORT.md` | runtime screenshot still useful, but binary evidence supports no frame-10 row for offline `0x102` |
| Combo/flag controls | not-touched | none | explicitly out of scope |
| Map preview decode | not-touched | none | explicitly out of scope |

## 7. Open Questions - Final State

- `[RESOLVED] Q1` - Which standard offline path reaches the shell? `FUN_006AE2C0` calls `FUN_0072CF40` then creates dialog `0x102` with proc `0x006AE3F0` (evidence: `0x006AE317..0x006AE328`).
- `[RESOLVED] Q2` - Which parent background globals are used by dialog `0x102`? `DAT_00B0FCE0`, `DAT_00B0FB50`, and `DAT_00B0FA18` (evidence: `FUN_0060CF00`, `FUN_0072D030`).
- `[RESOLVED] Q3` - Is `DAT_00B0FB50` `MNSCRNS.SHP` or `MNSCRNL.SHP`? `MNSCRNS.SHP`; `MNSCRNL.SHP` is `DAT_00B0FA04` (evidence: `0x0072EB50` load table and `0x00845144..0x00845150` strings).
- `[RESOLVED] Q4` - Are `SDTP.SHP`, `SDBTNBKGD.SHP`, `SDBTM.SHP` active for standard offline `0x102`? Yes, all are drawn by `RightPanel__Draw` (evidence: `0x0072E547`, `0x0072E594`, `0x0072E68C`).
- `[RESOLVED] Q5` - Is `SDBTNANM.SHP` visible in standard offline `0x102` right-panel chrome? No; loaded/palette-bound but frame-10 loop is skipped while the record byte remains default zero, and live setter xrefs are WOL-only (evidence: `0x00621FEC..0x00621FFE`, `0x0072E5E2..0x0072E65C`, `FUN_00608440` xrefs).
- `[RESOLVED] Q6` - Which palettes apply? `SHELL.PAL` for `SDTP/SDBTM/LWSCRN*`, `SHELL2.PAL` for `SDBTNBKGD`, `SDBTNANM.PAL` for conditional `SDBTNANM`, `MnScrnLCoopGameSetup.PAL` for parent background (evidence: `0x00844BE4..0x00844BEC`, `0x00845438..0x00845454`, `0x0072CF6A..0x0072CF7A`).
- `[DEFERRED] Q7` - Does a stale non-null `DAT_00B0FA18` ever survive into a non-800 fresh Skirmish entry? Static evidence says cleanup clears it and load only occurs at exact 800; runtime watchpoint is outside this slot (category: needs-runtime-debugger).

## Sources

- Ghidra decompile / assembly context: `0x00608440`, `0x0060CF00`, `0x00621E90`, `0x00622B50`, `0x00623340`, `0x006AE2C0`, `0x006AE3F0`, `0x0072ADE0`, `0x0072CF40`, `0x0072CF90`, `0x0072D030`, `0x0072DFB0`, `0x0072E260`, `0x0072E450`, `0x0072E730`, `0x0072EB50`, `0x0072EC70`, `0x004AED70`.
- Ghidra memory/table reads: `0x00844BE4`, `0x00845438`, `0x00844CD4`, `0x00845104`, `0x00844D6C`, `0x00844D70`, `0x00844F70`.
- Ghidra xrefs: `DAT_00B0FCDC`, `DAT_00B0FCE0`, `DAT_00B0FBCC`, `DAT_00B0FBD4`, `DAT_00B0FBDC`, `DAT_00B0FA18`, `FUN_00608440`.
- Cross-checked prior docs: `SKIRMISH_SHELL_RIGHT_PANEL_BACKGROUND_PALETTE_FOLLOWUP_GHIDRA_REPORT.md`, `SKIRMISH_SHELL_ACTIVE_RENDER_PATH_REINVESTIGATION_GHIDRA_REPORT.md`, `SHELL_PARENT_BSURFACE_COMPOSITION_AND_FLIP_GHIDRA_REPORT.md`, `traces/SKIRMISH_SHELL_CHROME_800X600_TRACE.md`, `SDBTNANM_FRAME10_OVERLAY_CONDITION_GHIDRA_REPORT.md`, `SDBTNANM_FRAME10_SETTER_CALLERS_GHIDRA_REPORT.md`.
