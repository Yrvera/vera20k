# Skirmish 0x102 Complete Child Rect Matrix - Ghidra Research Report

**Address(es):** `0x006AE2C0`, `0x0060C4A0`, `0x0060C0C0`, `0x00608CD0`, `0x00609730`, `0x00601360`, `0x0060B000`, `0x0060B1D0`, `0x0060B350`, `0x0060B550`, `0x0060B950`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** final visible child-window rectangles for standard offline Yuri's Revenge Skirmish setup dialog resource `0x102`, at 640x480, 800x600, and 1024x768, including helper branch for each child row.  
**Non-Scope:** owner-draw sub-rects inside child windows, dropdown popup/listbox geometry, Choose Map dialog `0x6B`, map preview marker projection, text colors, and input semantics except where needed to label active/conditional row visibility.  
**Confidence:** High for the matrix and helper branches; final rects combine binary RT_DIALOG resource extraction with verified resize/helper/fixup code.  
**Active in YR:** Yes for the offline `0x102` creation and resize path. AI row controls are Active in YR: Conditional, because map start count can hide rows after init; their final rect is verified for the child when visible.

## Working Notes

- Target question: What is the complete final child-control rect matrix for offline Skirmish setup dialog `0x102` at 640x480, 800x600, and 1024x768, and which resize helper branch owns each row?
- Non-goals: do not re-investigate Start/Choose/Back settled routing; do not investigate dropdown internals, text caller rects/colors, preview markers, or Choose Map `0x6B` internals.
- Evidence needed to mark COMPLETE: active offline launcher creates dialog `0x102`; RT_DIALOG `0x102` child inventory and DLU rects are extracted from `gamemd.exe`; resize dispatcher/helper/fixup functions are decompiled; final matrix includes every `WS_VISIBLE` child with helper branch and rects for all three modes; Rust-facing mismatches and do-not-do notes are recorded.
- Stop conditions: any unparsed visible child in resource `0x102`; any helper branch whose `0x102` control membership cannot be verified; Ghidra read-only access unavailable; evidence that a related dialog/control family is required to answer the matrix.

## 1. Overview

The active offline Skirmish launcher creates dialog resource `0x102`, resizes the parent window to the full screen, and enumerates all child windows through `ResizeShellChildControl_0060C0C0`. The child matrix is not scaled. Most children keep their RT_DIALOG pixel rectangle; only allowlisted right-panel children, the three owner-draw navigation buttons, tooltip/status `0x695`, and a small fixup set move.

All 72 children in RT_DIALOG `0x102` have `WS_VISIBLE` set in the resource. AI/opponent rows can later be hidden by map start-count logic, but the child windows are active standard-YR controls and their final rects below are the binary layout rects when the child remains visible.

## 2. Evidence Basis

| Evidence item | Finding | Active in YR | Evidence |
|---|---|---|---|
| Offline launcher | Creates dialog id `0x102` with proc `0x006AE3F0` | Yes | `FUN_006AE2C0`; assembly context `0x006AE317..0x006AE328` has `MOV ECX,0x102`, `MOV EDX,0x006AE3F0`, call `0x00622650` |
| Full-screen parent resize | Parent is moved to `g_ScreenWidth,g_ScreenHeight`, then children enumerate through `0x0060C0C0` | Yes | `FUN_0060C4A0` decompile |
| Resource inventory | RT_DIALOG id `0x102`, lang `1033`, file offset `0x4FF1E4`, size `3032`, 72 DIALOGEX children, font `MS Sans Serif` 8 | Yes | local binary parse of `C:/Users/enok/Documents/Command and Conquer Red Alert II/gamemd.exe`; DIALOGEX header |
| DLU conversion | Positive DLU values use base units `baseX=6`, `baseY=13`; pixel rects are `MulDiv(x,6,4)`, `MulDiv(y,13,8)`, etc. | Yes | RT_DIALOG font/header plus prior `SKIRMISH_CHECKBOX_TRACKBAR_PIXEL_GEOMETRY_GHIDRA_REPORT.md` and `SKIRMISH_COMBO_OWNERDRAW_GEOMETRY_GHIDRA_REPORT.md` |
| Start/Choose helper | Button metadata `+0x68==0` preempts right-anchor list and calls `FUN_0060B000` | Yes | dispatcher decompile; assembly context `0x0060C1B0..0x0060C1C8` |
| Back helper | Button metadata plus `FUN_00609730` routes `0x5C0` to `FUN_0060B350` | Yes | dispatcher decompile; assembly context `0x0060C213..0x0060C227` |
| Status helper | `0x695` routes through `FUN_0060B550` when `FUN_00601360(0x102)` is true | Yes | dispatcher decompile; assembly context `0x0060C2B6..0x0060C2C5` |
| One-pixel fixups | `0x694 y+1`, `0x50C y-1`, `0x54E/0x693/0x696/0x69A x-1`, `0x6A0 x+1,w+1` | Yes | `FUN_0060B950` decompile; assembly context `0x0060BE0A..0x0060BE66`, `0x0060C065..0x0060C092` |

## 3. Helper Branch Rules

| Branch | Controls in `0x102` | Formula summary | Active in YR |
|---|---|---|---|
| `FUN_0060B000 SDBTNANM snap` | `0x617`, `0x5AA` | `x = screen_w - max((screen_w-800)/2,0) - 156`; `w,h=156,42`; `y` snaps source top to nearest right-panel 42-px tile row | Yes |
| `FUN_0060B1D0 right-anchor` | `0x694`, `0x468`, `0x6EC`, `0x5A8` | Preserve size; `x = screen_w - x_center_offset - w - ((168-w)/2)`; `y = source_y + y_center_offset` | Yes |
| `FUN_0060B350 SDBTNANM bottom` | `0x5C0` | `x` as SDBTNANM right button; `y = last complete right-panel tile row`; `w,h=156,42` | Yes |
| `FUN_0060B550 bottom-left` | `0x695` | `x = x_center_offset + 10`; `y = screen_h - child_h - y_center_offset - 1` | Yes |
| fallback preserve | all other `0x102` children | Preserve DLU-derived pixel rect relative to full-screen parent | Yes |
| `FUN_0060B950` fixup | listed fixup controls only | Apply after whichever branch placed the child | Yes |

## 4. Complete Final Rect Matrix

`Resource px` is the DLU-derived child rect before resize-policy helpers. Final rects are `(x,y,w,h)`.

| # | ID | Class | Role/title | Active in YR | Resource px | Helper branch | 640x480 | 800x600 | 1024x768 |
|---:|---:|---|---|---|---:|---|---:|---:|---:|
| 0 | `0x5C0` | Button | `GUI:Back` | Yes | `(638,562,162,37)` | `FUN_0060B350 SDBTNANM bottom` | `(484,409,156,42)` | `(644,535,156,42)` | `(756,619,156,42)` |
| 1 | `0x694` | Static | `GUI:SkirmishGame` | Yes | `(638,2,162,16)` | `FUN_0060B1D0 right-anchor + FUN_0060B950 y+1` | `(475,3,162,16)` | `(635,3,162,16)` | `(747,87,162,16)` |
| 2 | `0x617` | Button | `GUI:StartGame` | Yes | `(638,242,162,37)` | `FUN_0060B000 SDBTNANM snap` | `(484,241,156,42)` | `(644,241,156,42)` | `(756,325,156,42)` |
| 3 | `0x6A0` | Edit | player name | Yes | `(57,59,150,23)` | `fallback preserve + FUN_0060B950 x+1,w+1` | `(58,59,151,23)` | `(58,59,151,23)` | `(58,59,151,23)` |
| 4 | `0x6A1` | ComboBox | side row 0 | Yes | `(287,59,117,120)` | `fallback preserve` | `(287,59,117,120)` | `(287,59,117,120)` | `(287,59,117,120)` |
| 5 | `0x50C` | Trackbar | unit count | Yes | `(404,341,128,21)` | `fallback preserve + FUN_0060B950 y-1` | `(404,340,128,21)` | `(404,340,128,21)` | `(404,340,128,21)` |
| 6 | `0x511` | Trackbar | credits | Yes | `(404,314,128,21)` | `fallback preserve` | `(404,314,128,21)` | `(404,314,128,21)` | `(404,314,128,21)` |
| 7 | `0x5AA` | Button | `GUI:ChooseMap` | Yes | `(638,286,162,37)` | `FUN_0060B000 SDBTNANM snap` | `(484,283,156,42)` | `(644,283,156,42)` | `(756,367,156,42)` |
| 8 | `0x696` | Button | `GUI:CratesAppear` | Yes | `(72,341,150,16)` | `fallback preserve + FUN_0060B950 x-1` | `(71,341,150,16)` | `(71,341,150,16)` | `(71,341,150,16)` |
| 9 | `0x693` | Button | `GUI:MCVRepacks` | Yes | `(72,314,150,16)` | `fallback preserve + FUN_0060B950 x-1` | `(71,314,150,16)` | `(71,314,150,16)` | `(71,314,150,16)` |
| 10 | `0x54E` | Button | `GUI:ShortGame` | Yes | `(72,286,150,16)` | `fallback preserve + FUN_0060B950 x-1` | `(71,286,150,16)` | `(71,286,150,16)` | `(71,286,150,16)` |
| 11 | `0x529` | Trackbar | game speed | Yes | `(404,286,128,21)` | `fallback preserve` | `(404,286,128,21)` | `(404,286,128,21)` | `(404,286,128,21)` |
| 12 | `0x699` | Static | `GUI:GameSpeed` | Yes | `(302,286,90,16)` | `fallback preserve` | `(302,286,90,16)` | `(302,286,90,16)` | `(302,286,90,16)` |
| 13 | `0x69B` | Static | `GUI:Credits` | Yes | `(302,314,90,16)` | `fallback preserve` | `(302,314,90,16)` | `(302,314,90,16)` | `(302,314,90,16)` |
| 14 | `0x69C` | Static | `GUI:UnitCount` | Yes | `(302,341,90,16)` | `fallback preserve` | `(302,341,90,16)` | `(302,341,90,16)` | `(302,341,90,16)` |
| 15 | `0x50B` | ComboBox | AI type row 1 | Conditional: hidden if selected map has too few starts | `(59,85,150,120)` | `fallback preserve` | `(59,85,150,120)` | `(59,85,150,120)` | `(59,85,150,120)` |
| 16 | `0x50E` | ComboBox | AI type row 2 | Conditional | `(59,111,150,120)` | `fallback preserve` | `(59,111,150,120)` | `(59,111,150,120)` | `(59,111,150,120)` |
| 17 | `0x510` | ComboBox | side row 1 | Conditional | `(287,85,117,120)` | `fallback preserve` | `(287,85,117,120)` | `(287,85,117,120)` | `(287,85,117,120)` |
| 18 | `0x516` | ComboBox | AI type row 3 | Conditional | `(59,137,150,120)` | `fallback preserve` | `(59,137,150,120)` | `(59,137,150,120)` | `(59,137,150,120)` |
| 19 | `0x51A` | ComboBox | AI type row 4 | Conditional | `(59,163,150,120)` | `fallback preserve` | `(59,163,150,120)` | `(59,163,150,120)` | `(59,163,150,120)` |
| 20 | `0x51B` | ComboBox | AI type row 5 | Conditional | `(59,189,150,120)` | `fallback preserve` | `(59,189,150,120)` | `(59,189,150,120)` | `(59,189,150,120)` |
| 21 | `0x51C` | ComboBox | AI type row 6 | Conditional | `(59,215,150,120)` | `fallback preserve` | `(59,215,150,120)` | `(59,215,150,120)` | `(59,215,150,120)` |
| 22 | `0x51D` | ComboBox | AI type row 7 | Conditional | `(59,241,150,120)` | `fallback preserve` | `(59,241,150,120)` | `(59,241,150,120)` | `(59,241,150,120)` |
| 23 | `0x514` | ComboBox | side row 4 | Conditional | `(287,163,117,120)` | `fallback preserve` | `(287,163,117,120)` | `(287,163,117,120)` | `(287,163,117,120)` |
| 24 | `0x51E` | ComboBox | side row 3 | Conditional | `(287,137,117,120)` | `fallback preserve` | `(287,137,117,120)` | `(287,137,117,120)` | `(287,137,117,120)` |
| 25 | `0x51F` | ComboBox | side row 5 | Conditional | `(287,189,117,120)` | `fallback preserve` | `(287,189,117,120)` | `(287,189,117,120)` | `(287,189,117,120)` |
| 26 | `0x520` | ComboBox | side row 6 | Conditional | `(287,215,117,120)` | `fallback preserve` | `(287,215,117,120)` | `(287,215,117,120)` | `(287,215,117,120)` |
| 27 | `0x522` | ComboBox | color row 1 | Conditional | `(423,85,44,119)` | `fallback preserve` | `(423,85,44,119)` | `(423,85,44,119)` | `(423,85,44,119)` |
| 28 | `0x523` | ComboBox | color row 2 | Conditional | `(423,111,44,119)` | `fallback preserve` | `(423,111,44,119)` | `(423,111,44,119)` | `(423,111,44,119)` |
| 29 | `0x524` | ComboBox | color row 3 | Conditional | `(423,137,44,119)` | `fallback preserve` | `(423,137,44,119)` | `(423,137,44,119)` | `(423,137,44,119)` |
| 30 | `0x525` | ComboBox | color row 4 | Conditional | `(423,163,44,119)` | `fallback preserve` | `(423,163,44,119)` | `(423,163,44,119)` | `(423,163,44,119)` |
| 31 | `0x526` | ComboBox | color row 5 | Conditional | `(423,189,44,119)` | `fallback preserve` | `(423,189,44,119)` | `(423,189,44,119)` | `(423,189,44,119)` |
| 32 | `0x527` | ComboBox | color row 6 | Conditional | `(423,215,44,119)` | `fallback preserve` | `(423,215,44,119)` | `(423,215,44,119)` | `(423,215,44,119)` |
| 33 | `0x6DA` | Static | flag row 0 | Yes | `(225,59,48,20)` | `fallback preserve` | `(225,59,48,20)` | `(225,59,48,20)` | `(225,59,48,20)` |
| 34 | `0x6DB` | Static | flag row 1 | Conditional | `(225,85,48,20)` | `fallback preserve` | `(225,85,48,20)` | `(225,85,48,20)` | `(225,85,48,20)` |
| 35 | `0x6DC` | Static | flag row 2 | Conditional | `(225,111,48,20)` | `fallback preserve` | `(225,111,48,20)` | `(225,111,48,20)` | `(225,111,48,20)` |
| 36 | `0x6DD` | Static | flag row 3 | Conditional | `(225,137,48,20)` | `fallback preserve` | `(225,137,48,20)` | `(225,137,48,20)` | `(225,137,48,20)` |
| 37 | `0x6DE` | Static | flag row 4 | Conditional | `(225,163,48,20)` | `fallback preserve` | `(225,163,48,20)` | `(225,163,48,20)` | `(225,163,48,20)` |
| 38 | `0x6DF` | Static | flag row 5 | Conditional | `(225,189,48,20)` | `fallback preserve` | `(225,189,48,20)` | `(225,189,48,20)` | `(225,189,48,20)` |
| 39 | `0x6E0` | Static | flag row 6 | Conditional | `(225,215,48,20)` | `fallback preserve` | `(225,215,48,20)` | `(225,215,48,20)` | `(225,215,48,20)` |
| 40 | `0x6E1` | Static | flag row 7 | Conditional | `(225,241,48,20)` | `fallback preserve` | `(225,241,48,20)` | `(225,241,48,20)` | `(225,241,48,20)` |
| 41 | `0x6EC` | Static | game-type text | Yes | `(648,167,135,16)` | `FUN_0060B1D0 right-anchor` | `(489,167,135,16)` | `(649,167,135,16)` | `(761,251,135,16)` |
| 42 | `0x695` | Static | status/tooltip blank | Yes | `(3,577,615,20)` | `FUN_0060B550 bottom-left` | `(10,459,615,20)` | `(10,579,615,20)` | `(122,663,615,20)` |
| 43 | `0x513` | ComboBox | side row 2 | Conditional | `(287,111,117,120)` | `fallback preserve` | `(287,111,117,120)` | `(287,111,117,120)` | `(287,111,117,120)` |
| 44 | `0x521` | ComboBox | side row 7 | Conditional | `(287,241,117,120)` | `fallback preserve` | `(287,241,117,120)` | `(287,241,117,120)` | `(287,241,117,120)` |
| 45 | `0x6A2` | ComboBox | color row 0 | Yes | `(423,59,44,119)` | `fallback preserve` | `(423,59,44,119)` | `(423,59,44,119)` | `(423,59,44,119)` |
| 46 | `0x528` | ComboBox | color row 7 | Conditional | `(423,241,44,119)` | `fallback preserve` | `(423,241,44,119)` | `(423,241,44,119)` | `(423,241,44,119)` |
| 47 | `0x468` | Static | preview placeholder | Yes | `(644,37,144,112)` | `FUN_0060B1D0 right-anchor` | `(484,37,144,112)` | `(644,37,144,112)` | `(756,121,144,112)` |
| 48 | `0x5A8` | Static | map label | Yes | `(648,189,135,33)` | `FUN_0060B1D0 right-anchor` | `(489,189,135,33)` | `(649,189,135,33)` | `(761,273,135,33)` |
| 49 | `0x69A` | Button | `GUI:SuperWeaponsAllowed` | Yes | `(72,371,155,16)` | `fallback preserve + FUN_0060B950 x-1` | `(71,371,155,16)` | `(71,371,155,16)` | `(71,371,155,16)` |
| 50 | `0x69D` | Button | `GUI:BuildOffAlly` | Yes | `(302,369,249,18)` | `fallback preserve` | `(302,369,249,18)` | `(302,369,249,18)` | `(302,369,249,18)` |
| 51 | `0x6A3` | ComboBox | start row 0 | Yes | `(486,59,38,119)` | `fallback preserve` | `(486,59,38,119)` | `(486,59,38,119)` | `(486,59,38,119)` |
| 52 | `0x6A4` | ComboBox | start row 1 | Conditional | `(486,85,38,119)` | `fallback preserve` | `(486,85,38,119)` | `(486,85,38,119)` | `(486,85,38,119)` |
| 53 | `0x6A5` | ComboBox | start row 2 | Conditional | `(486,111,38,119)` | `fallback preserve` | `(486,111,38,119)` | `(486,111,38,119)` | `(486,111,38,119)` |
| 54 | `0x6A6` | ComboBox | start row 3 | Conditional | `(486,137,38,119)` | `fallback preserve` | `(486,137,38,119)` | `(486,137,38,119)` | `(486,137,38,119)` |
| 55 | `0x6A7` | ComboBox | start row 4 | Conditional | `(486,163,38,119)` | `fallback preserve` | `(486,163,38,119)` | `(486,163,38,119)` | `(486,163,38,119)` |
| 56 | `0x6A8` | ComboBox | start row 5 | Conditional | `(486,189,38,119)` | `fallback preserve` | `(486,189,38,119)` | `(486,189,38,119)` | `(486,189,38,119)` |
| 57 | `0x6AA` | ComboBox | start row 6 | Conditional | `(486,215,38,119)` | `fallback preserve` | `(486,215,38,119)` | `(486,215,38,119)` | `(486,215,38,119)` |
| 58 | `0x6AB` | ComboBox | start row 7 | Conditional | `(486,241,38,119)` | `fallback preserve` | `(486,241,38,119)` | `(486,241,38,119)` | `(486,241,38,119)` |
| 59 | `0x76D` | ComboBox | team row 0 | Yes; enabled conditional on AlliesAllowed | `(546,59,38,119)` | `fallback preserve` | `(546,59,38,119)` | `(546,59,38,119)` | `(546,59,38,119)` |
| 60 | `0x76E` | ComboBox | team row 1 | Conditional | `(546,85,38,119)` | `fallback preserve` | `(546,85,38,119)` | `(546,85,38,119)` | `(546,85,38,119)` |
| 61 | `0x76F` | ComboBox | team row 2 | Conditional | `(546,111,38,119)` | `fallback preserve` | `(546,111,38,119)` | `(546,111,38,119)` | `(546,111,38,119)` |
| 62 | `0x770` | ComboBox | team row 3 | Conditional | `(546,137,38,119)` | `fallback preserve` | `(546,137,38,119)` | `(546,137,38,119)` | `(546,137,38,119)` |
| 63 | `0x771` | ComboBox | team row 4 | Conditional | `(546,163,38,119)` | `fallback preserve` | `(546,163,38,119)` | `(546,163,38,119)` | `(546,163,38,119)` |
| 64 | `0x772` | ComboBox | team row 5 | Conditional | `(546,189,38,119)` | `fallback preserve` | `(546,189,38,119)` | `(546,189,38,119)` | `(546,189,38,119)` |
| 65 | `0x773` | ComboBox | team row 6 | Conditional | `(546,215,38,119)` | `fallback preserve` | `(546,215,38,119)` | `(546,215,38,119)` | `(546,215,38,119)` |
| 66 | `0x774` | ComboBox | team row 7 | Conditional | `(546,241,38,119)` | `fallback preserve` | `(546,241,38,119)` | `(546,241,38,119)` | `(546,241,38,119)` |
| 67 | `0x796` | Static | `GUI:Players` | Yes | `(59,34,146,16)` | `fallback preserve` | `(59,34,146,16)` | `(59,34,146,16)` | `(59,34,146,16)` |
| 68 | `0x791` | Static | `GUI:Side` | Yes | `(287,34,110,16)` | `fallback preserve` | `(287,34,110,16)` | `(287,34,110,16)` | `(287,34,110,16)` |
| 69 | `0x792` | Static | `GUI:Color` | Yes | `(425,34,63,16)` | `fallback preserve` | `(425,34,63,16)` | `(425,34,63,16)` | `(425,34,63,16)` |
| 70 | `0x793` | Static | `GUI:StartPosition` | Yes | `(488,34,51,16)` | `fallback preserve` | `(488,34,51,16)` | `(488,34,51,16)` | `(488,34,51,16)` |
| 71 | `0x794` | Static | `GUI:Team` | Yes | `(545,34,51,16)` | `fallback preserve` | `(545,34,51,16)` | `(545,34,51,16)` | `(545,34,51,16)` |

## 5. Current Rust Implementation Status

Rust currently has most structural groups represented in `src/ui/skirmish_shell/layout.rs`: right-panel text, Start/Choose/Back, preview, column labels, player name, row combos, color combos, flags, trackbars, and checkboxes. The important current status is:

- As of `SKIRMISH_CHECKBOX_TRACKBAR_RECT_COLOR_RECHECK_GHIDRA_REPORT.md`, current Rust applies the verified `0x102` fixups: unit-count trackbar `0x50C` is `(404,340,128,21)`, the first four option checkboxes are x `71`, and BuildOffAlly remains x `302`.
- `layout.rs` does not expose status/tooltip static `0x695` or option-label statics `0x699/0x69B/0x69C` as named layout fields. If Rust renders those visible text surfaces, they need entries from the matrix above.
- `ShellControlId` in `layout.rs:51` is not a complete child id enum for dialog `0x102`; it omits many active controls. That is fine if it remains a subset, but it should not be used as the complete binary child inventory.

## 6. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Offline launcher and dialog id | verified | `0x006AE317..0x006AE328` | none |
| Parent full-screen resize and enumeration | verified | `FUN_0060C4A0` | none |
| RT_DIALOG child inventory | verified | `gamemd.exe` RT_DIALOG `0x102`, offset `0x4FF1E4`, 72 children | none |
| Right-anchor allowlist | verified | `FUN_00608CD0`; `0x102` case contains `0x6EC/0x5AA/0x5A8/0x617`, generic earlier case includes `0x694/0x468` | none |
| Start/Choose Button preemption | verified | `0x0060C1B0..0x0060C1C8` | none |
| Back bottom helper | verified | `0x0060C213..0x0060C227`, `FUN_00609730` | none |
| Status `0x695` bottom-left | verified | `0x0060C2B6..0x0060C2C5`, `FUN_0060B550` | none |
| Preserve fallback for ordinary controls | verified | `0x0060C396` fallback branch, parent id not in late-center list | none |
| `0x102` one-pixel fixups | verified | `FUN_0060B950`, assembly context `0x0060BE0A..0x0060BE66` and `0x0060C065..0x0060C092` | none |
| AI row dynamic show/hide | context-only | `SKIRMISH_PLAYER_AI_ROW_VISIBILITY_ENABLE_RULES_GHIDRA_REPORT.md` | dropdown/population semantics out of scope |

## 7. Open Questions - Final State

- `[RESOLVED] OQ-1 - Is dialog 0x102 active in standard offline YR? -> Yes; launcher passes id 0x102 and proc 0x006AE3F0 to the dialog creator.` (evidence: `0x006AE317..0x006AE328`)
- `[RESOLVED] OQ-2 - How many visible child controls are in RT_DIALOG 0x102? -> 72 children, all with WS_VISIBLE in the resource.` (evidence: `gamemd.exe` RT_DIALOG `0x102`, lang `1033`, offset `0x4FF1E4`)
- `[RESOLVED] OQ-3 - Which controls route through SDBTNANM button snap? -> Start `0x617` and Choose `0x5AA`.` (evidence: `FUN_00608CD0`, `0x0060C1B0..0x0060C1C8`)
- `[RESOLVED] OQ-4 - Which controls route through generic right-anchor? -> `0x694`, `0x468`, `0x6EC`, `0x5A8`.` (evidence: `FUN_00608CD0`, `FUN_0060B1D0`)
- `[RESOLVED] OQ-5 - Which controls route through bottom helpers? -> Back `0x5C0` uses `FUN_0060B350`; status `0x695` uses `FUN_0060B550`.` (evidence: `FUN_00609730`, `FUN_00601360`, dispatcher)
- `[RESOLVED] OQ-6 - Which ordinary controls receive 0x102 one-pixel fixups? -> `0x50C y-1`, `0x54E/0x693/0x696/0x69A x-1`, and `0x6A0 x+1,w+1`.` (evidence: `FUN_0060B950`)
- `[RESOLVED] OQ-7 - Do row/flag/color/start/team controls move at 1024x768? -> No; they use fallback preserve and remain at 800-shell resource coordinates.` (evidence: `0x0060C396` fallback and matrix)
- `[RESOLVED] OQ-8 - Are any visible 0x102 children missing from the matrix? -> No; all 72 RT_DIALOG children are represented exactly once.` (evidence: resource inventory vs Section 4)

## 8. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Unit-count trackbar `0x50C` final rect is `(404,340,128,21)`, not raw DLU `(404,341,128,21)` | `FUN_0060B950`; assembly context `0x0060BE0A..0x0060BE1B` tests parent `0x102`, control `0x50C`, then decrements Y | implemented/current as of 2026-05-23 | `src/ui/skirmish_shell/layout.rs`, trackbar tests | Preserve the post-resize y-1 fixup and dependent plaque/value/thumb geometry | At 800x600 and 1024x768, unit count trackbar face is one pixel above credits+27 raw DLU position; proposed test `skirmish_unit_count_trackbar_applies_0102_fixup_y_minus_one` | Do not preserve all three trackbars uniformly |
| Checkboxes `0x54E/0x693/0x696/0x69A` final x is `71`, while `0x69D` remains `302` | `FUN_0060B950`; assembly context `0x0060BE20..0x0060BE4A` tests the four ids and jumps to x decrement at `0x0060C065..0x0060C06A` | implemented/current as of 2026-05-23 | `src/ui/skirmish_shell/layout.rs`, checkbox hit/render paths in `state.rs` and `app_skirmish_shell_render.rs` | Preserve the first-four checkbox x-1 fixup and leave BuildOffAlly unchanged | At 800x600, Short Game icon hit starts at x=71 and label rect starts at x=97; proposed test `skirmish_option_checkboxes_apply_0102_fixup_x_minus_one` | Do not shift every checkbox; `0x69D` is not in the binary fixup set |
| Complete visible child inventory includes status `0x695` and option-label statics `0x699/0x69B/0x69C` | RT_DIALOG `0x102` inventory and helper matrix; `0x695` bottom-left at `0x0060C2B6..0x0060C2C5` | missing/unchecked named surfaces | `src/ui/skirmish_shell/layout.rs`, `src/app_skirmish_shell_render.rs` | Add explicit layout entries if Rust renders or hit-tests these text/static children; `0x695` should bottom-left anchor, option labels should preserve raw rects | At 640x480, status `0x695` rect is `(10,459,615,20)` and Game Speed label remains `(302,286,90,16)`; proposed test `skirmish_visible_static_children_complete_rect_matrix` | Do not infer completeness from `ShellControlId`; it is a subset |

### Stale Docs / Follow-up Docs

- `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_CHECKBOX_TRACKBAR_PIXEL_GEOMETRY_GHIDRA_REPORT.md`: replace Section 2 rows for `0x54E`, `0x693`, `0x696`, `0x69A`, and `0x50C` with: "`0x54E` `(71,286,150,16)`, `0x693` `(71,314,150,16)`, `0x696` `(71,341,150,16)`, `0x69A` `(71,371,155,16)`, and `0x50C` `(404,340,128,21)` after `ResizeShellChildControl_0060C0C0` calls `FUN_0060B950`; the RT_DIALOG/DLU resource values are one pixel different for these controls."
- `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_CHECKBOX_TRACKBAR_PIXEL_GEOMETRY_GHIDRA_REPORT.md`: replace checkbox sub-rect rows for those four checkboxes with icon x `71` and label-left x `97`; leave `0x69D` at icon x `302`, label-left x `328`.

### Negative Facts / Do Not Do

- Do not scale, center, or high-res offset the player/AI table, color combos, flags, start/team combos, option labels, or most checkboxes/trackbars. Active in YR: No for those transforms. Evidence: `0x0060C396` fallback preserve branch for `0x102`.
- Do not treat all checkboxes alike for the one-pixel fixup. Active in YR: No. Evidence: `FUN_0060B950` lists only `0x54E`, `0x693`, `0x696`, `0x69A`; `0x69D` is omitted.
- Do not keep all trackbars at identical DLU-preserve y positions. Active in YR: No. Evidence: `FUN_0060B950` applies `y-1` only to `0x50C`.
- Do not list Start/Choose as generic `FUN_0060B1D0` final-rect users. Active in YR: No for standard owner-draw Button metadata. Evidence: `0x0060C1B0..0x0060C1C8`.
- Do not include Choose Map dialog `0x6B` children in the `0x102` setup matrix. Active in YR: No for this parent id. Evidence: this report's RT_DIALOG `0x102` inventory and separate `0x6B` reports.

## Sources

- Ghidra read-only decompile / context: `0x006AE2C0`, `0x0060C4A0`, `0x0060C0C0`, `0x00608CD0`, `0x00609730`, `0x00601360`, `0x0060B000`, `0x0060B1D0`, `0x0060B350`, `0x0060B550`, `0x0060B950`.
- Ghidra assembly context: `0x006AE317..0x006AE328`, `0x0060C1B0..0x0060C1C8`, `0x0060C213..0x0060C227`, `0x0060C2B6..0x0060C2C5`, `0x0060BE0A..0x0060BE66`, `0x0060C065..0x0060C092`.
- Binary resource extraction: `C:/Users/enok/Documents/Command and Conquer Red Alert II/gamemd.exe`, RT_DIALOG type `5`, id `0x102`, lang `1033`, file offset `0x4FF1E4`, size `3032`.
- Prior docs cross-checked: `SKIRMISH_RESIZE_SHELL_CHILD_CONTROL_0060C0C0_COMPLETE_0X102_POLICY_GHIDRA_REPORT.md`, `SKIRMISH_CHECKBOX_TRACKBAR_PIXEL_GEOMETRY_GHIDRA_REPORT.md`, `SKIRMISH_COMBO_OWNERDRAW_GEOMETRY_GHIDRA_REPORT.md`, `SKIRMISH_PLAYER_AI_ROW_VISIBILITY_ENABLE_RULES_GHIDRA_REPORT.md`, `SKIRMISH_FLAG_STATICS_GHIDRA_REPORT.md`.
- Rust scanned read-only: `src/ui/skirmish_shell/layout.rs`, `src/ui/skirmish_shell/state.rs`, `src/app_skirmish_shell_render.rs`.
