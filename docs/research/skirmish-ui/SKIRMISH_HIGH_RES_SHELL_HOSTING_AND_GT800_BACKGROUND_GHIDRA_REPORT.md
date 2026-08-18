# Skirmish High-Resolution Shell Hosting and >800 Background - Ghidra Research Report

**Address(es):** `0x006AE2C0`, `0x00622650`, `0x00622B50`, `0x0060C4A0`, `0x0060B1D0`, `0x0060B350`, `0x00621E90`, `0x0072CF40`, `0x0072CF90`, `0x0072E450`, `0x0072E730`, `0x004AED70`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** Standard YR offline Skirmish dialog `0x102` shell host origin/size policy and parent background behavior when `g_ScreenWidth > 800`, including `MoveWindow`, child reposition helpers, `Background_Overlay`, `DAT_00B0FA18`, and `CC_Draw_Shape` null-pointer behavior.  
**Non-Scope:** Combo/control visuals, map preview decode, button PCX family drawing, WOL shell variants, and runtime screenshot capture. Controls are referenced only as layout anchors.  
**Confidence:** High for static binary behavior in this slice; Medium for stale non-null `DAT_00B0FA18` runtime history because the static standard Skirmish entry/cleanup path clears it, but no live watchpoint was used.  
**Active in YR:** Yes for the standard offline Skirmish `0x102` path. Conditional details are stated per finding.

## 1. Overview

Offline Skirmish does not keep the resource dialog as a fixed 800x600 window. The modeless child dialog is created under `g_hWnd`, then the common shell init path moves it to `(0,0,g_ScreenWidth,g_ScreenHeight)` and separately repositions selected children and shell draw rectangles around an 800x600 logical shell.

For `g_ScreenWidth > 800`, parent background drawing is not scaled. The draw rect is clipped to a centered 800x600-ish region, `Background_Overlay` selects the alternate/background-large pointer path, and `CC_Draw_Shape` returns immediately if the SHP pointer passed on that path is null.

## 2. Key Offsets and Globals

| Field/global | Meaning in this slice | Evidence | Active in YR |
|---|---|---|---|
| `g_ScreenWidth` / `g_ScreenHeight` (`0x008A00A4` / `0x008A00A8`) | Screen dimensions used to resize shell dialog and clamp draw rects | `0x0060C4A0`, `0x0072E730`, `0x0072E450` | Yes |
| Parent record `+0xB0` (`piVar9[0x2C]`) | Mode `1` selects right-panel/background shell paint branch | `WM_PAINT_Handler @ 0x00621E90`; `FUN_0060C540` sets mode for dialog id set including `0x102` | Yes |
| Parent record `+0x74` (`piVar1[0x1E]`) | Convert/palette pointer passed to `Background_Overlay` as ECX | `FUN_0060CF00`, `0x00622014..0x00622058`, `0x00622119` | Yes |
| Parent record `+0xE4` (`piVar1[0x3A]`) | Alternate/non-640 background SHP pointer for dialog `0x102`; assigned from `DAT_00B0FA18` | `FUN_0060CF00`, read before `0x0062211B` call | Yes; pointer may be zero when width is not exactly 800 |
| `DAT_00B0FA18` | `MnScrnLCoopGameSetup.shp` pointer loaded only at exact width 800, then cleared by cleanup | `0x0072CF49..0x0072CF65`, `0x0072CFCB` | Conditional: active/non-null only after exact-800 load unless some stale external history exists |
| `DAT_00B0FB50` | Small/640 parent SHP pointer used by dialog `0x102` at width 640 | `FUN_0060CF00`, `Sidebar_RightPanel_SHP_Loading @ 0x0072EB50` | Yes at 640 |
| `DAT_00B0FC1C` | Background destination rect, computed by `RightPanel__ComputeLayoutRects` | `0x0072ED34..0x0072ED3B`, read at `0x0072E795` | Yes after right-panel resource/layout init |

## 3. Core Logic

### 3.1 Standard offline Skirmish host path

**Finding:** Offline Skirmish reaches dialog `0x102` from `Main_Game`, creates it as a modeless child under `g_hWnd`, then pumps it until Start/Back.  
**Evidence:** `get_function_xrefs(FUN_006AE2C0)` reports `Main_Game @ 0x0052E168`; `FUN_006AE2C0` calls `FUN_0072CF40`, then `FUN_00622650`; `FUN_00622650` calls `CreateDialogIndirectParamA(..., g_hWnd, proc, ...)`. Prior resource report identified `0x102` as a child-style dialog.  
**Active in YR:** Yes. This is the standard offline Skirmish launcher path.

### 3.2 Parent shell HWND is full-screen top-left, not centered or scaled

**Finding:** During `WM_INITDIALOG`, `FUN_0060C540` returns true for the shell-dialog id set containing `0x102`; `FUN_0060C4A0` then calls `MoveWindow(hwnd,0,0,g_ScreenWidth,g_ScreenHeight,0)`. There is no parent-window centering transform in this function.  
**Evidence:** `FUN_0060C540 @ 0x0060C540` includes `iVar1 == 0x102`, sets `piVar3[0x2D]=1`, byte `+0xC1=1`, and returns true. `FUN_0060C4A0` decompile and assembly `0x0060C4A0..0x0060C4BA` read `g_ScreenHeight`, `g_ScreenWidth`, push `0, height, width, 0, 0, hwnd`, and call the `MoveWindow` import.  
**Active in YR:** Yes for standard offline Skirmish `0x102`.

### 3.3 Child repositioning is selective and uses centered 800x600 offsets

**Finding:** Child windows are not globally scaled after the parent resize. `ResizeShellChildControl_0060C0C0` selectively dispatches allowlisted children to anchoring helpers; unselected children are re-moved to their current parent-relative rectangle.  
**Evidence:** `FUN_0060C4A0` calls `EnumChildWindows(..., ResizeShellChildControl_0060C0C0, pair)`. The callback calls `FUN_00608CD0` before `FUN_0060B1D0` for right-panel anchors, and `FUN_00609730` before `FUN_0060B350`/`FUN_0060B420` for Back/bottom anchors; otherwise it falls through to a `MoveWindow` preserving the current rectangle relative to the resized parent.  
**Active in YR:** Yes. Specific control visual rendering remains out of scope.

**Finding:** `FUN_0060B1D0` uses `offset_x=max(0,(parent_width-800)/2)` and a vertical offset based on `max(0,(parent_height-600)/2)` minus the passed base-pair adjustment. For the standard `{640,480}` pair, the second term is clamped to zero, so larger screens add the centered 800x600 vertical offset.  
**Evidence:** `FUN_0060B1D0`: reads parent/child `GetWindowRect`, computes `local_24 = ((parent_width - DAT_007F5BE4)/2)` masked to zero when negative, computes vertical `local_28` similarly around `DAT_007F5BF0`, then `MoveWindow` uses `parent.right - inset - child_width - local_24` and `child.top-parent.top + local_28`.  
**Active in YR:** Yes for allowlisted right-panel Skirmish children.

**Finding:** `FUN_0060B350` anchors the Back button against the same centered 800-wide policy; it does not preserve resource `x`.  
**Evidence:** `FUN_0060B350`: when `FUN_0069BBE0()==0`, computes `uVar3=((parent_width-800)/2)` masked to zero when negative, sets `X = parent_width - offset_x - 0x9C`, and derives `Y` from `DAT_00B0FC24/DAT_00B0FC28`.  
**Active in YR:** Yes for the standard offline Back-button anchor when selected by `FUN_00609730`.

### 3.4 Parent paint order and draw clipping at widths greater than 800

**Finding:** In mode `1`, the parent paint path draws right-panel chrome first, then background overlay, then optional extras, then blits the cached parent `BSurface` to `DAT_00887310`.  
**Evidence:** `WM_PAINT_Handler @ 0x00621E90`: `RightPanel__Draw` call at `0x00621FFE`, `Background_Overlay` call at `0x0062211B`, optional calls from `0x00622120` onward, final blit starts at `0x00622396` by reading `DAT_00887310`.  
**Active in YR:** Yes for `0x102` mode `1` when the right-panel ready gate allows this branch.

**Finding:** `RightPanel__Draw` and `Background_Overlay` do not scale to full screen. Each copies the destination clip rect, and if width `>800` replaces right edge with `800 + (width-800)/2`; if height `>600` replaces bottom edge with `600 + (height-600)/2`. The left/top comes from the incoming rect, normally zero for the full-screen parent.  
**Evidence:** `RightPanel__Draw @ 0x0072E450` and `Background_Overlay @ 0x0072E730`; assembly in `Background_Overlay` at `0x0072E74F..0x0072E775` and `0x0072E779..0x0072E791`.  
**Active in YR:** Yes. At `1024x768`, this gives right/bottom clip limits `912,684`, matching an 800x600 logical shell centered inside the larger screen while keeping parent origin at `(0,0)`.

**Finding:** The background destination point/rect origin is centered by layout globals, not by moving the parent HWND. `RightPanel__ComputeLayoutRects` writes `DAT_00B0FC1C = {local_c, iVar6, shp_width, shp_height}`, where `local_c=(screen_w-800)/2` only if `screen_w>1023`, and `iVar6=(screen_h-600)/2` only if `screen_h>767`.  
**Evidence:** `RightPanel__ComputeLayoutRects @ 0x0072EC70`: `if (0x3FF < param_1) local_c=(param_1-800)/2`; `if (0x2FF < param_2) iVar6=(param_2-600)/2`; writes `DAT_00B0FC1C` at `0x0072ED34..0x0072ED3B`; `Background_Overlay` reads it at `0x0072E795`.  
**Active in YR:** Yes after right-panel initialization. Note the centering threshold is `>1023`/`>767` in layout computation, not merely `>800`/`>600`; the clip rect still clamps for any `>800`/`>600`.

### 3.5 `DAT_00B0FA18` and >800 parent background

**Finding:** `FUN_0072CF40` loads `DAT_00B0FA18` only when `g_ScreenWidth == 800`, not when `g_ScreenWidth > 800`. It still loads the Skirmish background palette/convert state on first guarded call.  
**Evidence:** Assembly `0x0072CF49` compares `g_ScreenWidth` to `0x320`; `0x0072CF53` jumps over the SHP load when not equal; `0x0072CF55..0x0072CF65` loads string pointer `0x00844D6C -> 0x00844FA8 "MnScrnLCoopGameSetup.shp"` into `DAT_00B0FA18`; `0x0072CF6A..0x0072CF7A` always loads `0x00844D70 -> 0x00844F8C "MnScrnLCoopGameSetup.PAL"` into `DAT_00B0FCDC/DAT_00B0FCE0`.  
**Active in YR:** Yes. The width condition is active in the standard offline Skirmish entry path.

**Finding:** `FUN_0060CF00` assigns dialog `0x102` parent background fields from `DAT_00B0FB50` and `DAT_00B0FA18`. Therefore at widths greater than 800, `Background_Overlay` selects the `+0xE4`/`DAT_00B0FA18` path even though the standard loader did not populate it for that width.  
**Evidence:** `FUN_0060CF00` branch for `0x102/0xBC/0xBD/0xC2/0xC9` calls `FUN_0072D030`, writes `piVar2[0x1E]`, `piVar2[0x39]=DAT_00B0FB50`, and `piVar2[0x3A]=DAT_00B0FA18`; `WM_PAINT_Handler` reads `+0xE4` at `0x00622108` and passes it to `Background_Overlay` at `0x0062211B`.  
**Active in YR:** Yes. The observed output at `>800` depends on whether that pointer is null or stale.

**Finding:** Standard Skirmish cleanup clears `DAT_00B0FA18`. A fresh `>800` entry after cleanup should therefore pass a null alternate-background pointer unless some non-standard stale runtime path preserves it.  
**Evidence:** `FUN_006AE2C0` calls `FUN_0072CF90` after the dialog is destroyed. `FUN_0072CF90` frees `DAT_00B0FA18` if the loaded-byte gate is set, then writes `DAT_00B0FA18=0` at `0x0072CFCB` and clears `DAT_00B0FCD9` at `0x0072CFFA`. `get_bulk_xrefs(00B0FA18)` found reads/writes at `0x0060D2A8`, `0x0072CF65`, `0x0072CFA5`, and `0x0072CFCB` for this global.  
**Active in YR:** Yes for the standard enter/exit lifecycle. Stale pointer visibility is conditional on abnormal ordering outside this slice.

### 3.6 `Background_Overlay` and `CC_Draw_Shape` null behavior

**Finding:** `Background_Overlay` chooses the small background only when `g_ScreenWidth == 640`; all other widths, including `800` and `>800`, choose the alternate pointer. It does not check that pointer before calling `CC_Draw_Shape`.  
**Evidence:** `Background_Overlay @ 0x0072E7A8..0x0072E815`: compare `g_ScreenWidth` to `0x280`; equal path pushes stack arg from the small pointer slot; non-equal path pushes stack arg from the alternate pointer slot and calls `0x004AED70`.  
**Active in YR:** Yes.

**Finding:** A null SHP pointer passed to `CC_Draw_Shape` is a visible no-op for this background path, not a crash and not a fallback image.  
**Evidence:** `CC_Draw_Shape @ 0x004AED70`: prologue moves the SHP pointer stack argument to `EDI` at `0x004AED79`, `TEST EDI,EDI` at `0x004AED84`, and jumps directly to return at `0x004AED8E -> 0x004AF289` if null. A second null test after lazy-load indirection at `0x004AEDAB..0x004AEDAD` also returns.  
**Active in YR:** Yes for all callers of `CC_Draw_Shape`, including Skirmish parent `Background_Overlay`.

**Finding:** A stale non-null `DAT_00B0FA18` would not be treated specially by `Background_Overlay` or `CC_Draw_Shape`; it would be drawn as whatever SHP pointer it contains. Static standard Skirmish lifecycle evidence clears the global, so this is a conditional hazard rather than normal standard-entry behavior.  
**Evidence:** `Background_Overlay` passes the pointer directly; `CC_Draw_Shape` only tests null/lazy-load state before interpreting the SHP. Cleanup zeroes `DAT_00B0FA18` at `0x0072CFCB`.  
**Active in YR:** Conditional. Active only if another live path leaves a non-null pointer despite the standard cleanup, which was not shown by this static slice.

## 4. INI Keys

No Skirmish-specific INI key controls the shell origin, parent resize, `DAT_00B0FA18` load gate, or `Background_Overlay` width branch in this slice. The effective inputs are global video dimensions (`ScreenWidth`, `ScreenHeight`, `AllowHiResModes` feed the globals in other code) and fixed shell resource strings.

## 5. Integration Points

| Integration point | Status | Evidence | Active in YR |
|---|---|---|---|
| `Main_Game -> FUN_006AE2C0` | verified | xref from `0x0052E168` | Yes |
| `FUN_006AE2C0 -> FUN_0072CF40 -> FUN_00622650` | verified | decompile of `0x006AE2C0` | Yes |
| `FUN_00622650 -> CreateDialogIndirectParamA(..., g_hWnd, ...)` | verified | decompile of `0x00622650` | Yes |
| `FUN_00622B50 WM_INITDIALOG -> FUN_0060C540/FUN_0060C4A0` | verified | decompile of `0x00622B50` | Yes |
| `FUN_0060C4A0 -> MoveWindow + EnumChildWindows` | verified | decompile and assembly `0x0060C4A0..0x0060C4C6` | Yes |
| `WM_PAINT_Handler -> RightPanel__Draw -> Background_Overlay -> blit` | verified | `0x00621FFE`, `0x0062211B`, `0x00622396` | Yes |
| `Background_Overlay -> CC_Draw_Shape` | verified | `0x0072E7B4..0x0072E815` | Yes |

## 6. Current Rust Implementation Status

Not scanned in this slot. The parent task limited this subagent to binary research and a single standalone report; no repo files were modified.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Standard offline Skirmish reachability | verified | `Main_Game @ 0x0052E168`, `FUN_006AE2C0` | none |
| Dialog creation host parent | verified | `FUN_00622650` `CreateDialogIndirectParamA(..., g_hWnd, ...)` | none |
| `FUN_0060C540` mode/fullscreen-shell gate | verified | dialog id set includes `0x102`, writes mode `1` | none |
| `FUN_0060C4A0` parent move policy | verified | `MoveWindow(hwnd,0,0,g_ScreenWidth,g_ScreenHeight,0)` | none |
| `ResizeShellChildControl_0060C0C0` child policy | verified for boundary | decompile dispatch to `FUN_0060B1D0`, `FUN_0060B350`, fallback preserve-rect move | exact visual internals out of scope |
| `FUN_0060B1D0` right anchor helper | verified | decompile formulas using 800/600 constants | none for origin policy |
| `FUN_0060B350` Back anchor helper | verified for x/origin policy | decompile formula `parent_width-offset_x-0x9C` | exact y/asset dimensions out of scope |
| `WM_PAINT_Handler` parent draw order | verified | `0x00621FFE`, `0x0062211B`, `0x00622396` | none |
| `RightPanel__Draw` >800 clip policy | verified | decompile of `0x0072E450` | none |
| `Background_Overlay` >800 selection/clip policy | verified | decompile/disassembly of `0x0072E730` | none |
| `DAT_00B0FA18` load/cleanup | verified | `0x0072CF49..0x0072CF65`, `0x0072CFCB` | live stale-pointer watch deferred |
| `CC_Draw_Shape` null SHP behavior | verified | `0x004AED79..0x004AED8E`, `0x004AEDAB..0x004AEDAD` | none |
| Runtime screenshot of `>800` shell | deferred | not a Ghidra static artifact | needs runtime capture if pixel proof is required |

## 8. Open Questions - Final State

- [RESOLVED] Q1 - Is the parent dialog fixed 800x600, centered, or transformed? It is moved to `(0,0,g_ScreenWidth,g_ScreenHeight)`; centering is applied by selected child/draw helpers, not by moving the parent. Evidence: `FUN_0060C4A0`, `0x0060C4A0..0x0060C4BA`.
- [RESOLVED] Q2 - Does dialog `0x102` enter the full-screen shell-host path? Yes. Evidence: `FUN_0060C540` id set includes `0x102` and `FUN_00622B50` calls `FUN_0060C4A0` when true.
- [RESOLVED] Q3 - Are child controls uniformly scaled? No; they are selectively moved by `ResizeShellChildControl_0060C0C0` helpers or preserved relative to the resized parent. Evidence: `ResizeShellChildControl_0060C0C0`.
- [RESOLVED] Q4 - What does `Background_Overlay` select when `g_ScreenWidth > 800`? The alternate pointer (`param_5`, parent `+0xE4`) with a centered/clamped draw rect. Evidence: `0x0072E74F..0x0072E815`.
- [RESOLVED] Q5 - Is `DAT_00B0FA18` loaded for widths greater than 800? No; the load is exact `g_ScreenWidth == 800`. Evidence: `0x0072CF49..0x0072CF65`.
- [RESOLVED] Q6 - What happens if the selected alternate SHP pointer is null? `CC_Draw_Shape` returns immediately, producing no background draw for that call. Evidence: `0x004AED79..0x004AED8E`.
- [RESOLVED] Q7 - Does standard cleanup clear `DAT_00B0FA18`? Yes. Evidence: `0x0072CFCB`, `0x0072CFFA`, and `FUN_006AE2C0` calls `FUN_0072CF90` after dialog teardown.
- [DEFERRED] Q8 - Can a stale non-null `DAT_00B0FA18` survive into a `>800` Skirmish entry in a live retail session? Category: needs-runtime-debugger. Static standard lifecycle says no, but proving every abnormal history would require watchpoints/runtime capture.
- [DEFERRED] Q9 - Exact pixels shown at `1024x768` or other high-res modes. Category: needs-runtime-debugger. Static evidence proves origin/clip/pointer behavior, not a screenshot.

## Sources

- Ghidra decompile/disassembly: `FUN_006AE2C0`, `FUN_00622650`, `FUN_00622B50`, `FUN_0060C540`, `FUN_0060C4A0`, `ResizeShellChildControl_0060C0C0`, `FUN_0060B1D0`, `FUN_0060B350`, `WM_PAINT_Handler @ 0x00621E90`, `FUN_0060CF00`, `FUN_0072CF40`, `FUN_0072CF90`, `FUN_0072E260`, `RightPanel__Draw @ 0x0072E450`, `Background_Overlay @ 0x0072E730`, `RightPanel__ComputeLayoutRects @ 0x0072EC70`, `CC_Draw_Shape @ 0x004AED70`.
- Ghidra xrefs/data: `FUN_006AE2C0` xref from `Main_Game @ 0x0052E168`; `DAT_00B0FA18` xrefs at `0x0060D2A8`, `0x0072CF65`, `0x0072CFA5`, `0x0072CFCB`; string pointer `0x00844D6C -> 0x00844FA8 "MnScrnLCoopGameSetup.shp"`; string pointer `0x00844D70 -> 0x00844F8C "MnScrnLCoopGameSetup.PAL"`.
- Prior docs cross-checked, not treated as ground truth: `SKIRMISH_0X102_COMMON_PARENT_PAINT_GHIDRA_REPORT.md`, `SKIRMISH_RIGHT_PANEL_SHELL_ASSET_PALETTE_SELECTION_GHIDRA_REPORT.md`, `SKIRMISH_SHELL_VIEWPORT_ORIGIN_GHIDRA_REPORT.md`, `SKIRMISH_OWNERDRAW_CALLBACKS_FOLLOWUP_GHIDRA_REPORT.md`.
