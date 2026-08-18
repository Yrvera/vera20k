# Skirmish High-Resolution Shell Hosting Origin - Ghidra Research Report

**Address(es):** `0x006AE2C0`, `0x00622650`, `0x00622B50`, `0x0060C540`, `0x0060C4A0`, `0x0060C0C0`, `0x00608CD0`, `0x00609730`, `0x0060B1D0`, `0x0060B350`, `0x0072EC70`, `0x0072E450`, `0x0072E730`, `0x004AED70`
**Investigation Mode:** exhaustive-slice, scoped to dialog/client hosting origin and 640/800/>800 placement policy
**Claimed Scope:** standard offline Skirmish dialog resource `0x102`; final parent HWND placement; child-coordinate baseline; selective post-create transforms for right-panel controls; right-panel/background viewport implications at 640x480, 800x600, and modes larger than 800x600
**Non-Scope:** combo owner-draw internals, flag rendering, map marker asset identity, button PCX drawing, online/WOL variants, and live screenshot capture
**Confidence:** High for static binary formulas and active standard YR path; Medium for 640x480 final player-visible composition because it is formula-verified but not screenshot-captured in this slot
**Active in YR:** Yes. The launcher path is the standard offline Skirmish shell path, not a TS-only legacy path.

## 1. Overview

Offline Skirmish `0x102` is created from a Win32 dialog resource, but the final shell parent is not kept at the resource-created 800x600 size. Common shell initialization moves the Skirmish dialog HWND to `(0,0,g_ScreenWidth,g_ScreenHeight)` under `g_hWnd`, then selectively repositions only allowlisted children and shell chrome around an 800x600 logical shell.

The practical result is mixed: unselected child controls keep their DLU-derived 800-layout pixel coordinates inside the full-screen parent, while right-panel controls and shell chrome use screen-size-aware formulas. At `>800` modes, the parent remains top-left/full-screen and the 800x600 content is centered by helper formulas and draw rects, not by a centered parent HWND.

## 2. Coordinate Baseline

Resource `0x102` is `DIALOGEX 0,0,533,369` with `MS Sans Serif`, 8 pt. Prior PE-resource extraction established `baseX=6`, `baseY=13`, so DLU conversion is:

| Axis | Formula | Active in YR |
|---|---|---|
| X / width | `MulDiv(dlu, 6, 4)` | Yes; Win32 dialog template creation path |
| Y / height | `MulDiv(dlu, 13, 8)` | Yes; Win32 dialog template creation path |

This maps the nominal resource to `800x600`. Example: Start button `0x617` resource `(425,149,108,23)` becomes `(638,242,162,37)` before post-create right anchoring. The `x=638` comes from Win32 rounding of `425*6/4 = 637.5`.

## 3. Core Logic

### 3.1 Standard Skirmish reachability and host parent

| Finding | Evidence | Active in YR |
|---|---|---|
| Offline Skirmish reaches `FUN_006AE2C0`, loads Skirmish shell resources, creates dialog `0x102`, pumps until Start `0x617` or Back `0x5C0`, then cleans up. | `FUN_006AE2C0`: calls `FUN_0072CF40`, then `FUN_00622650`; loop exits on `0x617`/`0x5C0`; cleanup calls `FUN_0072CF90`. | Yes; standard offline Skirmish launcher. |
| The dialog is modeless and hosted directly under `g_hWnd`. | `FUN_00622650`: obtains dialog template via `FUN_004A3B40`, calls `CreateDialogIndirectParamA(hInstance, template, g_hWnd, proc, &local_8)`. | Yes. |
| Dialog `0x102` enters the full-screen shell host set. | `FUN_0060C540`: dialog id comparison set includes `0x102`, writes shell-mode fields, returns true. `FUN_00622B50` calls `FUN_0060C4A0` when this returns nonzero. | Yes. |

### 3.2 Parent HWND origin and size

The parent Skirmish dialog HWND is full-screen top-left in the main shell client:

```text
MoveWindow(hwnd, 0, 0, g_ScreenWidth, g_ScreenHeight, 0)
```

Evidence: `FUN_0060C4A0` decompile. It then writes `DAT_00AC48A8 = hwnd` and calls `EnumChildWindows(hwnd, ResizeShellChildControl_0060C0C0, pair)`.

Active in YR: Yes for standard offline `0x102`.

This resolves the top-level host question:

| Screen mode | Parent dialog HWND | Active in YR |
|---|---|---|
| 640x480 | `(0,0,640,480)` | Yes if global video mode is 640x480 |
| 800x600 | `(0,0,800,600)` | Yes |
| `>800x600`, e.g. 1024x768 | `(0,0,W,H)` | Yes if high-res video mode is active |

The parent HWND itself is neither centered nor scaled inside larger modes.

### 3.3 Child-control transform policy

`ResizeShellChildControl_0060C0C0` does not apply a uniform post-create scale. It selects branches:

| Branch | Effect | Evidence | Active in YR |
|---|---|---|---|
| `FUN_00608CD0` true | right-panel anchoring through `FUN_0060B1D0` | `ResizeShellChildControl_0060C0C0`; `FUN_00608CD0` includes `0x102` + `0x468`, `0x6EC`, `0x5AA`, `0x5A8`, `0x617` plus title path `0x694` | Yes |
| `FUN_00609730` true for owner-draw Back | bottom/right anchoring through `FUN_0060B350` | `FUN_00609730`: for dialog `0x102`, returns true for control `0x5C0` | Yes |
| fallback | preserves current child rect relative to parent | `ResizeShellChildControl_0060C0C0`: computes child rect minus parent rect, calls `MoveWindow` with same width/height | Yes |

Unselected controls such as slot-table combos and flag statics retain their DLU-derived pixel coordinates inside the resized parent. Active in YR: Yes, as fallback behavior of the standard child enumeration callback.

### 3.4 Right-panel anchoring formulas

For controls selected by `FUN_00608CD0`, `FUN_0060B1D0` computes:

```text
ox = max(0, (parent_width  - 800) / 2)
oy = max(0, (parent_height - 600) / 2) - max(0, (base_pair_height - 600) / 2)
inset = child_record.+0xE0 if nonzero else (168 - child_width) / 2
x = parent_width - ox - child_width - inset
y = original_child_y + oy
```

For the standard `{640,480}` pair passed by `FUN_00622B50`, the second vertical term clamps to zero. Active in YR: Yes for `0x102` right-panel children.

For Back `0x5C0`, `FUN_0060B350` computes:

```text
ox = max(0, (parent_width - 800) / 2)
x = parent_width - ox - 156
width  = SDBTNANM.SHP.width  = 156
height = SDBTNANM.SHP.height = 42
y = fc24.y + (tile_count - 1) * fc24.h
```

Evidence: `FUN_0060B350`, plus prior follow-up asset-header verification for `SDBTNANM.SHP=156x42` and `SDBTNBKGD.SHP=168x42`. Active in YR: Yes.

### 3.5 Right-panel/background draw viewport

`RightPanel__ComputeLayoutRects` centers shell chrome only past threshold boundaries:

| Formula | Evidence | Active in YR |
|---|---|---|
| if `screen_w > 1023`, `left_margin=(screen_w-800)/2`; otherwise `0` | `RightPanel__ComputeLayoutRects @ 0x0072EC70` checks `0x3FF < param_1` | Yes |
| if `screen_h > 767`, `top_margin=(screen_h-600)/2`; otherwise `0` | `RightPanel__ComputeLayoutRects @ 0x0072EC70` checks `0x2FF < param_2` | Yes |
| right panel x is `effective_right - 168` | `RightPanel__ComputeLayoutRects` writes `DAT_00B0FC20/24/28` using loaded SHP width | Yes |

`RightPanel__Draw` and `Background_Overlay` additionally clamp the draw clip right/bottom when the incoming rect is wider than `800` or taller than `600`, but they do not move the parent HWND. Active in YR: Yes.

At `>800`, `Background_Overlay` selects the non-640 alternate background pointer. Prior high-res report verified `DAT_00B0FA18` is loaded only for exact `g_ScreenWidth == 800`; when null, `CC_Draw_Shape @ 0x004AED70` returns immediately. Active in YR: Yes; non-null stale-pointer behavior is conditional and not normal standard-entry evidence.

## 4. Final Placement Summary

These are shell-client/backbuffer coordinates after dialog creation, full-screen host resize, and verified child layout helpers. The 640 values are formula results from active code, not live screenshot captures.

| Surface/control | 640x480 | 800x600 | 1024x768 example | Policy | Active in YR |
|---|---:|---:|---:|---|---|
| Parent dialog HWND | `(0,0,640,480)` | `(0,0,800,600)` | `(0,0,1024,768)` | `FUN_0060C4A0` full-screen top-left | Yes |
| Right panel top `SDTP` | `(472,0,168,199)` | `(632,0,168,199)` | `(744,84,168,199)` | `RightPanel__ComputeLayoutRects` | Yes |
| Right panel tile rect | `(472,199,168,42)` | `(632,199,168,42)` | `(744,283,168,42)` | `RightPanel__ComputeLayoutRects` | Yes |
| Right panel bottom cap | `(472,451,168,29)` | `(632,577,168,23)` | `(744,661,168,23)` | `RightPanel__ComputeLayoutRects` | Yes |
| Start `0x617` | `(475,242,162,37)` | `(635,242,162,37)` | `(747,326,162,37)` | `FUN_0060B1D0`, default inset 3 | Yes |
| Choose Map `0x5AA` | `(475,286,162,37)` | `(635,286,162,37)` | `(747,370,162,37)` | `FUN_0060B1D0`, default inset 3 | Yes |
| Map preview `0x468` | `(484,37,144,112)` | `(644,37,144,112)` | `(756,121,144,112)` | `FUN_0060B1D0`, default inset 12 | Yes |
| Back `0x5C0` | `(484,409,156,42)` | `(644,535,156,42)` | `(756,619,156,42)` | `FUN_0060B350` + `SDBTNANM/SDBTNBKGD` dimensions | Yes |
| Slot-table unselected controls | retain 800-layout px positions, e.g. first flag `(225,59,48,20)` | same | same unless individually allowlisted | fallback branch preserves child rect | Yes |

Implication: modeling the Skirmish UI as one uniformly centered 800x600 dialog is wrong. The parent is full-screen; selected right-panel/chrome surfaces are centered or right-anchored by helper code; ordinary controls are not globally recentered.

## 5. INI Keys

No Skirmish-specific INI key was found in the prior origin reports or this focused verification that changes dialog origin, DLU conversion, child anchoring, or shell viewport placement. The relevant inputs are global video dimensions (`g_ScreenWidth`, `g_ScreenHeight`) populated by the general video option path (`[Video] ScreenWidth`, `[Video] ScreenHeight`, `AllowHiResModes`).

Active in YR: Conditional for high-res modes; the layout code is active, while availability of a given mode depends on global video configuration.

## 6. Integration Points

| Integration | Status | Evidence | Active in YR |
|---|---|---|---|
| `Main_Game -> FUN_006AE2C0` | verified by prior xref and current launcher decompile | `FUN_006AE2C0` standard shell sequence | Yes |
| `FUN_006AE2C0 -> FUN_0072CF40 -> FUN_00622650` | verified | current Ghidra decompile | Yes |
| `FUN_00622650 -> CreateDialogIndirectParamA(..., g_hWnd, ...)` | verified | current Ghidra decompile | Yes |
| `FUN_00622B50 WM_INITDIALOG -> FUN_0060C540 -> FUN_0060C4A0` | verified | current Ghidra decompile | Yes |
| `FUN_0060C4A0 -> MoveWindow + EnumChildWindows` | verified | current Ghidra decompile | Yes |
| `ResizeShellChildControl_0060C0C0 -> FUN_0060B1D0/FUN_0060B350/fallback` | verified | current Ghidra decompile | Yes |
| `RightPanel__ComputeLayoutRects -> RightPanel__Draw/Background_Overlay` | verified | current Ghidra decompile | Yes |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Standard offline Skirmish reachability | verified | `FUN_006AE2C0`; prior xref from `Main_Game @ 0x0052E168` | none |
| Dialog modeless host parent | verified | `FUN_00622650` | none |
| DLU-to-pixel baseline | verified from prior PE resource report | `SKIRMISH_OWNERDRAW_CALLBACKS_FOLLOWUP_GHIDRA_REPORT.md`; resource `0x102` | none |
| Full-screen host gate | verified | `FUN_0060C540`, `FUN_00622B50` | none |
| Parent top-left/full-screen move | verified | `FUN_0060C4A0` | none |
| Selective child transform callback | verified | `ResizeShellChildControl_0060C0C0` | none for hosting/origin |
| Right-panel child allowlist for `0x102` | verified | `FUN_00608CD0` | unrelated controls out of scope |
| Back button allowlist for `0x102` | verified | `FUN_00609730` | none |
| Right-panel helper formula | verified | `FUN_0060B1D0` | none |
| Back helper formula | verified | `FUN_0060B350`; prior asset dimensions | none for origin/rect |
| Right-panel layout rect thresholds | verified | `RightPanel__ComputeLayoutRects @ 0x0072EC70` | none |
| Draw clip policy | verified | `RightPanel__Draw @ 0x0072E450`, `Background_Overlay @ 0x0072E730` | exact screenshots deferred |
| 640x480 visible composition | touched-not-exhausted | formulas verified in active functions | live screenshot/pixel capture if required |
| `>800` background pointer null behavior | verified from prior report and spot check | `Background_Overlay`, `CC_Draw_Shape @ 0x004AED70` | stale-pointer live watch deferred |

## 8. Open Questions - Final State

- [RESOLVED] Q1 - Is the final Skirmish parent fixed 800x600? No. It is moved to `(0,0,g_ScreenWidth,g_ScreenHeight)`. Evidence: `FUN_0060C4A0`.
- [RESOLVED] Q2 - Is the final Skirmish parent centered in high-res modes? No. Centering happens in selected child/chrome formulas, not by parent HWND movement. Evidence: `FUN_0060C4A0`, `FUN_0060B1D0`, `RightPanel__ComputeLayoutRects`.
- [RESOLVED] Q3 - Are child controls uniformly scaled or transformed after dialog creation? No. `ResizeShellChildControl_0060C0C0` selectively anchors allowlisted controls and otherwise preserves the existing child rect. Evidence: `0x0060C0C0`.
- [RESOLVED] Q4 - What is the DLU origin implication? The DLU template gives an 800x600 baseline at parent origin `(0,0)`; after parent resize, ordinary controls remain at those pixel positions while allowlisted right-panel controls move. Evidence: resource docs plus `ResizeShellChildControl_0060C0C0`.
- [RESOLVED] Q5 - How does `>800` centering work? `FUN_0060B1D0` uses `(W-800)/2` and `(H-600)/2` offsets for selected children; `RightPanel__ComputeLayoutRects` centers chrome at thresholds `W>1023`, `H>767`; draw helpers clamp right/bottom clip at `800+(W-800)/2`, `600+(H-600)/2`. Evidence: `0x0060B1D0`, `0x0072EC70`, `0x0072E450`, `0x0072E730`.
- [DEFERRED] Q6 - Does a retail 640x480 screenshot show any additional visual clipping or oddities beyond the verified formulas? Category: needs-runtime-debugger. Static code resolves formulas, but screenshot capture was not part of this slot.
- [DEFERRED] Q7 - Can abnormal shell history leave `DAT_00B0FA18` stale-non-null before `>800` Skirmish entry? Category: needs-runtime-debugger. Static standard lifecycle cleanup clears it; exhaustive runtime history was out of scope.

## Sources

- Ghidra decompiled this slot: `FUN_006AE2C0`, `FUN_00622650`, `FUN_00622B50`, `FUN_0060C540`, `FUN_0060C4A0`, `ResizeShellChildControl_0060C0C0`, `FUN_00608CD0`, `FUN_00609730`, `FUN_0060B1D0`, `FUN_0060B350`, `RightPanel__ComputeLayoutRects @ 0x0072EC70`, `RightPanel__Draw @ 0x0072E450`, `Background_Overlay @ 0x0072E730`, `CC_Draw_Shape @ 0x004AED70`.
- Prior docs checked:
  - `docs/research/skirmish-ui/SKIRMISH_SHELL_LAYOUT_ASSETS_GHIDRA_REPORT.md`
  - `docs/research/skirmish-ui/SKIRMISH_SHELL_CHROME_800X600_TRACE.md`
  - `docs/research/skirmish-ui/SKIRMISH_OWNERDRAW_CALLBACKS_FOLLOWUP_GHIDRA_REPORT.md`
  - `docs/research/skirmish-ui/SKIRMISH_SHELL_VIEWPORT_ORIGIN_GHIDRA_REPORT.md`
  - `docs/research/skirmish-ui/SKIRMISH_SHELL_VIEWPORT_ORIGIN_FOLLOWUP_GHIDRA_REPORT.md`
  - `docs/research/skirmish-ui/SKIRMISH_HIGH_RES_SHELL_HOSTING_AND_GT800_BACKGROUND_GHIDRA_REPORT.md`
