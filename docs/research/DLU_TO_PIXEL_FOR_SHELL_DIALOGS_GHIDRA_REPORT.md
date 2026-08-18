# DLU-to-Pixel Conversion for Shell Dialogs — Ghidra Research Report

**Date:** 2026-05-19  
**Primary addresses:** `FUN_00622650 @ 0x00622650`, `FUN_00622800 @ 0x00622800`  
**Confidence:** High  
**Active in YR:** Yes — fires on every shell dialog creation including 0xE2 and 0x120  
**Scope:** Narrow per brief — verify baseX=6/baseY=13, identify Win32 API used, confirm no post-creation re-scale of DLU units.

---

## 1. Overview

Shell dialogs (0xE2, 0x120, etc.) are created by `FUN_00622650` which wraps
`CreateDialogIndirectParamA`. The DLU-to-pixel conversion is performed entirely
inside Win32's `CreateDialogIndirectParamA`, using the font declared in the
`DS_SETFONT` template header (MS Sans Serif 8pt). The engine never calls
`GetDialogBaseUnits` or `MapDialogRect` — neither is in the import table.
After creation, the dialog parent window is expanded to full-screen via
`FUN_0060C4A0 (MoveWindow only)`, and children are repositioned by
`ResizeShellChildControl_0060C0C0`. Neither step re-derives or overrides
the template font or DLU base units.

---

## 2. Key Findings

### Finding 1 — Win32 API used for DLU-to-pixel

`FUN_00622650 @ 0x00622650` calls:

```
FindResourceA(hInstance, (LPCSTR)(param_1 & 0xFFFF), RT_DIALOG=5)
LoadResource → LockResource → lpTemplate
CreateDialogIndirectParamA(hInstance, lpTemplate, g_hWnd, dlgProc, lParam)
```

Verified via: `decompile_function 0x00622650` + `read_memory 0x00622650 80`
(bytes at +0x17: `BA 05 00 00 00` = `MOV EDX, 5` = RT_DIALOG type;
bytes at +0x19: `E8 CF 14 E8 FF` = `CALL FUN_004a3b40`).

Win32 `CreateDialogIndirectParamA` with a `DS_SETFONT` template internally
creates the dialog font and derives base units from it. This is standard Win32
behavior — the engine simply relies on it without any custom metric computation.

**`GetDialogBaseUnits` and `MapDialogRect` are absent from the entire import
table.** Confirmed via exhaustive `list_imports` scan (360 entries checked,
no match). The engine performs no explicit DLU-to-pixel calculation itself.

---

### Finding 2 — Dialog font for 0xE2 and 0x120

Both templates declare font `MS Sans Serif, 8pt` with `DS_SETFONT` in the
template header.

- 0xE2: `DIALOGEX`, style `0x40000040 (WS_CHILD|DS_SETFONT)`, confirmed
  in `MAIN_MENU_DIALOG_0XE2_FULL_VISIBLE_COMPOSITION_GHIDRA_REPORT.md`.
- 0x120: `DLGTEMPLATE`, style `0x40000040 (WS_CHILD|DS_SETFONT)`, font
  `MS Sans Serif 8pt`, confirmed in `RT_DIALOG_0X120_RESOURCE_LAYOUT_GHIDRA_REPORT.md`
  (bytes 0x18-0x35 of the blob at VA `0x00C00B24`).

No runtime font override: `FUN_00622B50 @ 0x00622B50` (common shell
`WM_INITDIALOG` handler, decompiled this session) has **no `WM_SETFONT (0x30)`
send** to the parent or any child. The only font-related path in that function
is `GetStockObject(4)` returned for `WM_CTLCOLOR*` messages (0x132–0x138),
which sets the brush for child background painting — it does not alter the
dialog font used for DLU measurement.

---

### Finding 3 — baseX=6, baseY=13 confirmed

Standard Win32 formula: `x_px = MulDiv(x_dlu, baseX, 4)`, `y_px = MulDiv(y_dlu, baseY, 8)`.

Arithmetic verification (Python `muldiv(a,b,c) = (a*b + c//2) // c`):

- `muldiv(533, 6, 4) = 800` ✓
- `muldiv(369, 13, 8) = 600` ✓
- `muldiv(425, 6, 4) = 638` ✓ (doc-cited x for button column)
- `muldiv(108, 6, 4) = 162` ✓ (doc-cited cx for buttons)

Only `baseX=6, baseY=13` produces `800×600` from `533×369 DLU`. No other
combination of baseX ∈ {6,7,8} and baseY ∈ {12,13,14} produces that result.
This is the value Win32 uses for MS Sans Serif 8pt at 96 DPI.

---

### Finding 4 — FUN_00622800: no DLU work, no font override

`FUN_00622800 @ 0x00622800` decompiles to:

```c
ShowWindow(param_1, 1);
SetForegroundWindow(param_1);
FUN_0054f720();
```

`FUN_0054f720` is a network-message poll (reads fields at `+0x314`, `+0x318`,
calls `Process_NetworkMessages`). No `MoveWindow`, no `SetWindowPos`, no
`WM_SETFONT`, no reposition. This function does nothing that affects DLU-to-pixel
conversion.

---

### Finding 5 — FUN_0060C4A0 expands parent only; no DLU re-derivation

`FUN_0060C4A0 @ 0x0060C4A0` decompiles to:

```c
MoveWindow(param_1, 0, 0, g_ScreenWidth, g_ScreenHeight, 0);
DAT_00AC48A8 = param_1;
EnumChildWindows(param_1, ResizeShellChildControl_0060C0C0, param_2);
```

`MoveWindow` on the parent changes the parent's window rect but does **not**
affect the dialog font or the pixel positions already assigned to child controls
by `CreateDialogIndirectParamA`. Children were already created and positioned
using baseX=6/baseY=13; `EnumChildWindows` then re-anchors specific children
(0x695, 0x71D, 0x694) per the RESIZESHELLCHILDCONTROL doc. No DLU math is
redone — the resize helpers operate in pixels, not DLUs.

---

### Finding 6 — No per-control DLU-to-pixel overrides beyond the 3 verified helpers

`ResizeShellChildControl_0060C0C0` (decompiled in RESIZESHELLCHILDCONTROL doc)
calls exactly: `FUN_0060B550` (0x695), `FUN_0060B610` (0x71D), `FUN_0060B950`
(all, pixel-nudge finalizer), and coord-space fixup `MoveWindow` for all others.
None of these functions contain `MulDiv` calls or DLU constants (4, 8, baseX,
baseY). Confirmed via full decompilation of all seven move-math helpers in the
RESIZESHELLCHILDCONTROL report — all math is pixel arithmetic against
`g_ScreenWidth/Height` and the `168`-px sidebar constant.

There are no per-control DLU-to-pixel overrides. The 3 helpers in the
COMPOSITION doc (`FUN_0060B550`, `FUN_0060B610`, `FUN_0060B950`) are the
complete set of post-creation repositioners for dialog 0xE2.

---

## 3. Open Questions — Final State

- `[RESOLVED] Q1` — Which Win32 API converts DLU to pixels? →
  `CreateDialogIndirectParamA` internally, using the `DS_SETFONT` template font.
  Engine never calls `GetDialogBaseUnits` or `MapDialogRect`.
  (evidence: `list_imports` exhaustive scan; `decompile_function 0x00622650`)

- `[RESOLVED] Q2` — Dialog font for 0xE2 and 0x120? →
  MS Sans Serif 8pt in both templates; not overridden at runtime.
  (evidence: prior resource parse docs; `decompile_function 0x00622B50` —
  no `WM_SETFONT` send found)

- `[RESOLVED] Q3` — baseX=6, baseY=13 verified? →
  Yes — the only base unit pair that maps 533×369 DLU → 800×600 px.
  Formulas: `x_px = MulDiv(x_dlu, 6, 4)`, `y_px = MulDiv(y_dlu, 13, 8)`.
  (evidence: arithmetic verification; all cited pixel positions in prior doc
  confirmed consistent)

- `[RESOLVED] Q4` — Does FUN_0060C4A0 post-creation expansion change DLU-to-px
  for children? → No. `MoveWindow` on parent + `EnumChildWindows` pixel
  repositioning only. DLU measurement already complete inside `CreateDialogIndirectParamA`.
  (evidence: `decompile_function 0x0060C4A0`)

- `[RESOLVED] Q5` — Per-control DLU-to-px overrides beyond the 3 verified
  reposition helpers? → None found. All seven move-math helpers in
  `ResizeShellChildControl_0060C0C0` work in pixels. No `MulDiv` with DLU
  base constants present.
  (evidence: `RESIZESHELLCHILDCONTROL_AND_REPOS_HELPERS_GHIDRA_REPORT.md`;
  `decompile_function 0x0060C4A0`, `decompile_function 0x00622B50`)

---

## 4. Sources

Ghidra MCP calls (read-only, this session):

- `decompile_function 0x00622650` → FUN_00622650 (CreateDialogIndirectParamA wrapper)
- `decompile_function 0x00622800` → FUN_00622800 (ShowWindow/SetForeground/net poll)
- `decompile_function 0x004a3b40` → resource loader (FindResourceA/LoadResource/LockResource)
- `decompile_function 0x00531cc0` → main menu entry point (verifies 0xE2 dialog ID)
- `decompile_function 0x00622B50` → common shell WM_INITDIALOG handler (no font override)
- `decompile_function 0x0060C4A0` → fullscreen expand (MoveWindow + EnumChildWindows)
- `decompile_function 0x00777080` → CenterChildWindow (pixel math only)
- `decompile_function 0x0054f720` → network poll (confirms FUN_00622800 is inert)
- `decompile_function 0x005d4e70` → dialog stack push (inert for DLU purposes)
- `read_memory 0x00622650 80` → verified RT_DIALOG=5, dialog ID param, call sites
- `read_memory 0x00531cc0 32` → verified MOV ECX, 0xE2 before FUN_00622650 call
- `list_imports` (all 360 entries) → confirmed GetDialogBaseUnits and MapDialogRect absent
- `get_function_callers 0x00622650` → 30 call sites, none relevant to DLU metrics
- `search_functions GetDialogBaseUnits / MapDialogRect / GetDialogBase` → all returned 0 matches

Prior reports cross-referenced:

- `MAIN_MENU_DIALOG_0XE2_FULL_VISIBLE_COMPOSITION_GHIDRA_REPORT.md`
- `RT_DIALOG_0X120_RESOURCE_LAYOUT_GHIDRA_REPORT.md`
- `RESIZESHELLCHILDCONTROL_AND_REPOS_HELPERS_GHIDRA_REPORT.md`
