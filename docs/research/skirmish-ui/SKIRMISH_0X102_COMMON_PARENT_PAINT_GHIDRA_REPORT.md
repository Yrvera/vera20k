# Skirmish Dialog 0x102 Common Parent Paint - Ghidra Research Report

**Address(es):** `0x006AE2C0`, `0x006AE3F0`, `0x00622B50`, `0x00621E90`, `0x0072CF40`, `0x0060CF00`, `0x0072E450`, `0x0072E730`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** Active standard Yuri's Revenge offline Skirmish dialog `0x102` common parent paint path: resource preload, common dialog proc delegation, parent `WM_PAINT`, parent background/chrome ordering, and child-control paint boundary.  
**Non-Scope:** Combo-box internals, flag PCX mapping, map preview decode, button PCX geometry, command handling, and non-Skirmish shell dialogs except where a shared function branch is a paint-order boundary.  
**Confidence:** High for the named slice. Medium only for the exact runtime first-paint value of the `DAT_00B0FBE0` right-panel-ready gate because this pass used read-only static Ghidra, not a live breakpoint.  
**Active in YR:** Yes for the offline Skirmish `0x102` path; conditional where explicitly gated below.

## 1. Overview

The standard offline Skirmish launcher reaches dialog resource `0x102` and installs dialog proc `0x006AE3F0`. That proc delegates every message to the common shell proc `FUN_00622B50` before doing Skirmish-specific work; for `WM_PAINT`, the common proc calls `WM_PAINT_Handler @ 0x00621E90`.

For the `0x102` mode-1 parent record, `WM_PAINT_Handler` composes into a cached parent `BSurface`: right-panel chrome first, then the parent background overlay, then optional generic extras, then one blit to `DAT_00887310`. Skirmish-specific preview/start-position painting in `0x006AE3F0` happens only after the common handler returns.

## 2. Key Offsets and Globals

| Field/global | Meaning in this slice | Evidence | Active in YR |
|---|---|---|---|
| Parent record `+0x14` (`EBX+0x10` after record+4 alias) | Cached parent `BSurface` pointer, allocated lazily on first unsuppressed paint | `0x00621F4D..0x00621F9B`, `WM_PAINT_Handler` | Yes, for dialogs with metadata records including `0x102` |
| Parent record `+0x20` (`EBX+0x20`) | Paint suppress/re-entry guard; nonzero skips composition and allocation | `0x00621F44..0x00621F47` | Conditional; active if init/reconfigure code sets it |
| Parent record `+0xB0` (`EBX+0xB0`) | Paint mode; value `1` selects common shell right-panel/background branch | `0x00621FB1..0x00621FBA`; `0x0060C540` writes `piVar3[0x2d]=1` for dialog id list containing `0x102` | Yes for `0x102` |
| Parent record `+0x74` | Convert/palette object for background overlay | `0x0060D294..0x0060D299`; later read at `0x0062204E..0x00622055` | Yes for `0x102` |
| Parent record `+0xE0` | Small/640 background SHP pointer, set to `DAT_00B0FB50` | `0x0060D29C..0x0060D2A2`; later read at `0x006220F5` | Yes for `0x102` |
| Parent record `+0xE4` | Alternate/non-640 background SHP pointer, set to `DAT_00B0FA18` | `0x0060D2A8..0x0060D2AE`; later read at `0x00622108` | Yes for `0x102`, but populated by `0x0072CF40` only at exact width 800 |
| `DAT_00B0FCD9` | Skirmish background/palette loader guard | `0x0072CF40..0x0072CF86` | Yes; once set, `0x0072CF40` returns without reloading |
| `DAT_00B0FBE0` | Right-panel resource-ready flag returned by `0x0072E260`; if zero, the mode-1 parent branch skips right-panel/background overlay work | `0x0072E260`, branch `0x00621FDF..0x00621FE6` | Conditional; standard shell code uses the right-panel path when this flag is nonzero |

## 3. Core Logic

### 3.1 Standard Skirmish reachability

**Finding:** The standard offline Skirmish launcher creates dialog resource `0x102` with proc `0x006AE3F0`, after calling the Skirmish paired background/palette loader.

**Evidence:** In `FUN_006AE2C0`, assembly context at `0x006AE317..0x006AE328` calls `0x0072CF40`, moves `EDX=0x006AE3F0`, `ECX=0x102`, pushes `0`, and calls `0x00622650`. `FUN_00622650` passes those values to `CreateDialogIndirectParamA`.  
**Active in YR:** Yes. This is the offline Skirmish setup launcher path and pumps the dialog until Start `0x617` or Back `0x5C0`.

### 3.2 `0x0072CF40` resource load order

**Finding:** `0x0072CF40` is guarded by `DAT_00B0FCD9`. On first run, it loads `MnScrnLCoopGameSetup.shp` only when `g_ScreenWidth == 800`, then always loads/converts `MnScrnLCoopGameSetup.PAL`, and finally sets the guard byte.

**Evidence:** Disassembly `0x0072CF40..0x0072CF86`: guard test at `0x0072CF40..0x0072CF47`; exact `CMP [g_ScreenWidth],0x320` at `0x0072CF49`; SHP pointer table read `0x00844D6C` at `0x0072CF55`; `DAT_00B0FA18` write at `0x0072CF65`; PAL pointer table read `0x00844D70` at `0x0072CF6A`; `0x0072ADE0` call with `DAT_00B0FCDC/DAT_00B0FCE0` at `0x0072CF70..0x0072CF7A`; guard set at `0x0072CF7F`. String evidence: `0x00844FA8 = "MnScrnLCoopGameSetup.shp"`, `0x00844F8C = "MnScrnLCoopGameSetup.PAL"`.  
**Active in YR:** Yes for offline Skirmish, because `0x006AE2C0` calls it before creating dialog `0x102`. Conditional detail: the SHP load is exact-width `800`, not `>= 800`; the PAL path is not width-gated.

### 3.3 Dialog `0x102` parent background fields

**Finding:** `FUN_0060CF00` assigns dialog `0x102` the convert pointer from `0x0072D030`, `DAT_00B0FB50` as parent `+0xE0`, and `DAT_00B0FA18` as parent `+0xE4`.

**Evidence:** `FUN_0060CF00` decompile contains the dialog id test including `0x102`. Assembly context `0x0060D294..0x0060D2AE`: call `0x0072D030`, write `[ESI+0x74]`, read `DAT_00B0FB50`, write `[ESI+0xE0]`, read `DAT_00B0FA18`, write `[ESI+0xE4]`.  
**Active in YR:** Yes. `FUN_00622B50` calls `FUN_0060CF00` during `WM_INITDIALOG` when `FUN_0069BBE0()` is false; standard offline Skirmish uses the non-WOL/local branch.

**Correction to prior docs:** `DAT_00B0FB50` maps to `MNSCRNS.SHP`, not `MNSCRNL.SHP`, in the right-panel loader sequence examined here. `Sidebar_RightPanel_SHP_Loading @ 0x0072EB50` loads pointer table `0x00844CE0 -> 0x00845150 "MNSCRNS.SHP"` via call at `0x0072EB9A`, and stores that result to `DAT_00B0FB50` at `0x0072EBAA`. Pointer table `0x00844CE4 -> 0x00845144 "MNSCRNL.SHP"` is loaded by the following call at `0x0072EBAF` and stored to `DAT_00B0FA04` at `0x0072EBBF`, not to `DAT_00B0FB50`.  
**Active in YR:** Yes as a data-flow correction for the active `0x102` parent background pointer.

### 3.4 `FUN_00622B50` common paint delegation

**Finding:** On `WM_PAINT (0x0F)`, `FUN_00622B50` finds the parent metadata record. If parent byte `+0xC0` is set, it validates the full rect and returns `1`; otherwise it calls `WM_PAINT_Handler`, handles a generic child-update flag `+0xBE`, validates the parent rect, and returns `0`.

**Evidence:** Decompile of `0x00622B50` `case 0xF`; direct call to `WM_PAINT_Handler` at `0x00622C4F`; full-parent `ValidateRect` before return.  
**Active in YR:** Yes. `0x006AE3F0` calls `0x00622B50` first for every message, with assembly context `0x006AE404..0x006AE411` showing call then early return if nonzero.

### 3.5 Parent `WM_PAINT_Handler` composition order

**Finding:** In mode `+0xB0 == 1`, the parent composition order is:

1. Allocate/reuse parent `BSurface` at record `+0x14`.
2. If `FUN_0069BBE0()` is false and `FUN_0072E260()` returns nonzero, call `RightPanel__Draw`.
3. Re-read parent `+0x74`, `+0xE0`, and `+0xE4`.
4. Call `Background_Overlay`.
5. Optionally call `Sidebar_TopHighlight`, `Minimap_Button`, and `RadarBackground` if their bytes are set.
6. Blit the cached parent `BSurface` to `DAT_00887310`.

**Evidence:** `WM_PAINT_Handler @ 0x00621E90`: allocation block `0x00621F4D..0x00621F9B`; mode test `0x00621FB1..0x00621FBA`; right-panel call at `0x00621FFE`; background overlay call at `0x0062211B`; optional calls at `0x00622130`, `0x00622145`, `0x006221C7`; final blit through `DAT_00887310->vtable+8` at `0x00622396..0x006223B3`.  
**Active in YR:** Yes for dialog `0x102` when the common shell mode-1 branch is unsuppressed. Conditional details: WOL/alternate branch `FUN_0069BBE0()!=0` diverts to `LeftPanel__Draw`; `DAT_00B0FBE0==0` skips the right-panel/background overlay section.

### 3.6 Right-panel chrome internal order

**Finding:** `RightPanel__Draw @ 0x0072E450` draws the common right-panel stack in this order: `SDTP`, repeated `SDBTNBKGD`, optional repeated `SDBTNANM` frame `10` when its boolean parameter is zero, `SDBTM`, then width-selected lower side piece (`DAT_00B0FAE8` at width 640, otherwise `DAT_00B0FA54`).

**Evidence:** Decompile of `0x0072E450`; calls to `CC_Draw_Shape` in that sequence; width branch `if (g_ScreenWidth == 0x280)` before final lower-piece draw.  
**Active in YR:** Yes for mode-1 common shell parent paint when the `DAT_00B0FBE0` ready gate is nonzero. The optional `SDBTNANM` frame-10 overlay is conditional on the byte passed from parent record `+0xD4`.

### 3.7 Parent background selection

**Finding:** `Background_Overlay @ 0x0072E730` selects the small pointer (`param_4`, parent `+0xE0`) only when `g_ScreenWidth == 640`; otherwise it selects the alternate pointer (`param_5`, parent `+0xE4`). For dialog `0x102`, that means `MNSCRNS.SHP` at 640 and `MnScrnLCoopGameSetup.shp` at 800.

**Evidence:** `0x0072E730` decompile: `if (g_ScreenWidth == 0x280) CC_Draw_Shape(param_4...) else CC_Draw_Shape(param_5...)`. Parent parameter mapping comes from `0x00622055` (`+0x74`), `0x006220F5` (`+0xE0`), `0x00622108` (`+0xE4`), then pushes before call at `0x0062211B`. `DAT_00B0FB50 -> MNSCRNS.SHP` mapping is from `0x0072EB9A/0x0072EBAA`; `DAT_00B0FA18 -> MnScrnLCoopGameSetup.shp` is from `0x0072CF55/0x0072CF65`.  
**Active in YR:** Yes for 640 and 800 standard widths. Conditional/unresolved for widths greater than 800: `Background_Overlay` selects `+0xE4`, but `0x0072CF40` only freshly populates `DAT_00B0FA18` at exact width 800.

### 3.8 Child-control paint boundary

**Finding:** Parent background/chrome composition is not performed by child controls. `FUN_0060F9A0` subclasses child/control windows during init and stores target owner-draw callbacks separately; the parent `WM_PAINT` path composes and blits the parent surface first, and `0x006AE3F0` only performs Skirmish-specific preview/start-position work after the common handler returns.

**Evidence:** `FUN_00622B50` `WM_INITDIALOG` enumerates children through `FUN_0060F9A0` before parent mode setup and resize (`0x00622F2C..0x00622FBE` and mirrored branch `0x00623030..0x00623063`). `FUN_0060F9A0` installs a common WndProc with `SetWindowLongA(..., -4, 0x610CA0)` and stores class-specific callbacks, including `OwnerDraw_Button_00612B70` for qualifying Button styles. `0x006AE3F0` assembly calls `0x00622B50` first at `0x006AE40A`, then its `WM_PAINT` branch calls `DrawStartPositions @ 0x00640710` only at `0x006AE47B` after common paint returned zero.  
**Active in YR:** Yes. This is the standard dialog proc and child-subclass setup path for `0x102`. Button/combo drawing internals are out of scope here.

## 4. INI Keys

No INI key participates directly in this parent paint ordering slice. The material inputs are dialog resource id `0x102`, screen width globals, shell resource globals, and retail assets loaded through fixed string tables.

## 5. Integration Points

| Integration point | Status | Evidence | Active in YR |
|---|---|---|---|
| Offline Skirmish launcher -> resource loader -> dialog create | Verified | `0x006AE317..0x006AE328` | Yes |
| Dialog proc -> common shell proc first | Verified | `0x006AE404..0x006AE411` | Yes |
| Common `WM_PAINT` -> `WM_PAINT_Handler` | Verified | `0x00622C4F` | Yes, unless parent suppress byte `+0xC0` returns early |
| Common parent paint -> Skirmish preview branch | Verified | `0x006AE454..0x006AE483` after `0x00622B50` returns zero | Yes when preview object `DAT_00AC1154 != 0` and child `0x468` is not suppressing draw |
| Child owner-draw callbacks | Boundary only | `FUN_0060F9A0` installs child callbacks; no child PCX geometry traced | Yes, but internals out of scope |

## 6. Current Rust Implementation Status

Current Rust has a dev-gated Skirmish shell renderer, not the default setup screen: `src/app.rs:1208` calls `render_skirmish_shell` only when `state.dev_skirmish_shell_enabled` is true.

The renderer has a semantic draw-order helper at `src/app_skirmish_shell_render.rs:467`. As of this scan, that helper pushes parent background before right-panel roles (`src/app_skirmish_shell_render.rs:476` before `:483`), while gamemd's parent composition calls `RightPanel__Draw` before `Background_Overlay`.

The Rust atlas currently loads `MNSCRNS.SHP` and `MnScrnLCoopGameSetup.shp` as parent backgrounds (`src/render/skirmish_shell_chrome.rs:137`) and stores them as `background_640_mnscrns` / `background_800_coop_game_setup` (`src/render/skirmish_shell_chrome.rs:33`, `:207`). That matches this report's corrected `DAT_00B0FB50 -> MNSCRNS.SHP` mapping.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `FUN_006AE2C0` standard Skirmish launcher | verified | `0x006AE317..0x006AE328` | none for this slice |
| `FUN_00622650` dialog creation | verified | decompile: `CreateDialogIndirectParamA` with caller-supplied `0x102`/proc | none |
| `FUN_0072CF40` Skirmish background/palette loader | verified | disassembly `0x0072CF40..0x0072CF86` | none for 640/800; >800 output deferred |
| `FUN_006AE3F0` common-first dialog proc | verified | `0x006AE40A`, `0x006AE454..0x006AE483` | none |
| `FUN_00622B50` `WM_PAINT` path | verified | `0x00622C4F`; decompile case `0xF` | none |
| `WM_PAINT_Handler @ 0x00621E90` mode-1 order | verified | `0x00621FFE`, `0x0062211B`, `0x00622396..0x006223B3` | live value of ready gate not runtime-confirmed |
| `RightPanel__Draw @ 0x0072E450` order | verified | decompile draw sequence | exact geometry/frame clipping out of scope |
| `Background_Overlay @ 0x0072E730` width selection | verified | decompile width branch | >800 null/stale alternate pointer behavior deferred |
| Child owner-draw/control paint internals | deferred | boundary evidence in `FUN_0060F9A0` | out-of-scope: combo/button PCX internals |
| Prior-doc `MNSCRNL` claim for parent `+0xE0` | conflict-needs-resolution | current Ghidra maps `DAT_00B0FB50` to `MNSCRNS.SHP` | update or verify prior docs in a separate doc-audit slot |

## 8. Open Questions - Final State

- [RESOLVED] Q1 - Does standard offline Skirmish reach dialog `0x102` with proc `0x006AE3F0`? Yes. Evidence: `0x006AE31C..0x006AE328`.
- [RESOLVED] Q2 - Does Skirmish call `FUN_0072CF40` before dialog creation? Yes. Evidence: `0x006AE317` before `0x006AE328`.
- [RESOLVED] Q3 - What does `FUN_0072CF40` load for this slice? `MnScrnLCoopGameSetup.shp` only at exact width 800; `MnScrnLCoopGameSetup.PAL` always on first guarded call. Evidence: `0x0072CF49..0x0072CF7F`.
- [RESOLVED] Q4 - Which parent fields does dialog `0x102` receive? `+0x74=DAT_00B0FCE0`, `+0xE0=DAT_00B0FB50`, `+0xE4=DAT_00B0FA18`. Evidence: `0x0060D294..0x0060D2AE`.
- [RESOLVED] Q5 - Which asset is `DAT_00B0FB50` in the current Ghidra trace? `MNSCRNS.SHP`. Evidence: pointer table `0x00844CE0 -> 0x00845150`, load/call/store at `0x0072EB8A..0x0072EBAA`.
- [RESOLVED] Q6 - What is the common parent paint order? Right panel, then background overlay, optional extras, final blit. Evidence: `0x00621FFE`, `0x0062211B`, optional calls after `0x00622120`, final blit `0x00622396..0x006223B3`.
- [RESOLVED] Q7 - Does Skirmish-specific preview painting occur before or after common parent paint? After. Evidence: `0x006AE40A` common call precedes `WM_PAINT` branch; `DrawStartPositions` call at `0x006AE47B`.
- [RESOLVED] Q8 - Are child controls part of the parent background/chrome draw? No; they are subclassed/control-boundaries through `FUN_0060F9A0`, with separate callbacks. Evidence: `FUN_0060F9A0` SetWindowLong/callback storage; parent paint calls no child owner-draw PCX routine except the out-of-scope generic child update message for control `0x71A`.
- [DEFERRED] Q9 - What does the player see at widths greater than 800 if `+0xE4` is stale/null? Category: out-of-scope. Reason: user constrained this slot to common paint ordering; resolving >800 output needs runtime screenshot or lower-level `CC_Draw_Shape` null/stale-pointer behavior.
- [DEFERRED] Q10 - Exact first-paint value of parent byte `+0xD4` controlling `SDBTNANM` frame-10 overlay. Category: out-of-scope. Reason: this slot treats button/right-panel detailed geometry as a boundary only.

## Sources

- Ghidra decompile/disassembly: `0x006AE2C0`, `0x00622650`, `0x006AE3F0`, `0x00622B50`, `0x00621E90`, `0x0072CF40`, `0x0072CF90`, `0x0060CF00`, `0x0060C540`, `0x0060C4A0`, `0x0060F9A0`, `0x0072E260`, `0x0072D030`, `0x0072E450`, `0x0072E730`, `0x0072EB50`.
- Ghidra string/data checks: `0x00844FA8`, `0x00844F8C`, `0x00845144`, `0x00845150`, pointer tables `0x00844CE0`, `0x00844CE4`, `0x00844D6C`, `0x00844D70`.
- Prior docs cross-checked, not treated as ground truth: `SKIRMISH_SHELL_ACTIVE_RENDER_PATH_REINVESTIGATION_GHIDRA_REPORT.md`, `SKIRMISH_SHELL_ACTIVE_RENDER_PATH_LIVE_GHIDRA_REPORT.md`, `SKIRMISH_SHELL_BACKGROUND_TEXT_PREVIEW_GHIDRA_REPORT.md`, `SHELL_PARENT_BSURFACE_COMPOSITION_AND_FLIP_GHIDRA_REPORT.md`, `traces/SKIRMISH_SHELL_CHROME_800X600_TRACE.md`.
- Rust scan references: `src/app.rs`, `src/app_skirmish_shell_render.rs`, `src/render/skirmish_shell_chrome.rs`.
