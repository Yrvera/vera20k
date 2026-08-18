# ResizeShellChildControl and Reposition Helpers — Ghidra Report

Date: 2026-05-19

Scope: `ResizeShellChildControl_0060C0C0 @ 0x0060C0C0` plus the four helpers named
in the parent investigation brief (`FUN_0060CAF0`, `FUN_0060C930`, `FUN_0060CCC0`,
`FUN_0060CDB0`). Full decompilation of every move-math function reached by the
dispatcher for dialog `0xE2` children. Caller chain back to `FUN_00531CC0` /
`FUN_0052B9B0`. Control-ID-to-helper mappings verified from live decompilation,
not from inference.

No Rust code, INI files, or Ghidra annotations were modified.

Active in YR: Yes — the entire resize path fires unconditionally for `0xE2` at
non-800×600 resolutions whenever `FUN_0060C540` selects the fullscreen-expand
branch (which it does for `0xE2`).

---

## CORRECTION (2026-05-30): `FUN_00608CD0` Decompiled — Buttons Take `FUN_0060B000`, Not Fallback

This report's premise that `FUN_00608CD0` "does not return non-zero for `0xE2`" (and the
resulting routing of the six buttons + title to the fallback coord-fixup) is **WRONG**.
The predicate was never decompiled when this report was written; it has since been read in
full (`decompile_function 0x00608CD0`).

Verified truth table for parent dialog `0xE2`: `FUN_00608CD0` returns **true** for
`0x683, 0x684, 0x578, 0x686, 0x55C` (and `0x55F, 0x694, 0x71C`), and **false** for Exit
`0x3EE`. Consequences:

- The five non-Exit buttons have control-kind 0 (`decompile_function 0x0060F9A0`:
  `piVar14[0x1a] = local_ab0 = 0` for the `(style&0xB)==0xB` branch), satisfying the
  `piVar1[0x1b]==0` gate, so they take **`FUN_0060B000`** — resize to the `SDBTNANM.SHP`
  cell (156×42), x=644 (flush right), grid-snap Y. `FUN_0060B000` **IS** reached for `0xE2`.
- Exit `0x3EE` (`FUN_00608CD0` false) is the **only** button on the fallback path; it
  keeps its raw template rect (≈162×37 @ x=638, y=536, no resize/snap).
- Title `0x694` and static `0x71C` (`FUN_00608CD0` true, not button-style) take
  **`FUN_0060B1D0`** → sidebar inset, so the title's final X is **635**, not the raw 638
  implied below. `FUN_0060B1D0` **IS** reached for `0xE2`.

Full analysis: `MAIN_MENU_0XE2_BUTTON_PAINT_AND_REPOSITION_FORK_GHIDRA_REPORT.md`. The
sections below are left for history; load-bearing claims are corrected inline.

---

## Correction: The Four "Immediate Helpers" Are Not Resize Math

`FUN_0060CAF0`, `FUN_0060C930`, `FUN_0060CCC0`, and `FUN_0060CDB0` are
**dialog-type classifier predicates**, not reposition helpers. Each takes a window
handle, looks up the window's dialog-record slot in a hash table (keyed off HWND,
table root at `DAT_00AC1B00`), reads field `[+0x1C]` (the dialog type/class id),
and writes a boolean byte into the record before returning 0 or 1.

| Function | Record byte written | Dialog IDs that set it `1` (i.e., return `1`) |
|---|---|---|
| `FUN_0060CAF0` | `[+0xD9]` | `0xBC, 0xBD, 0x102, 0xC2, 0xC9, 0xBC6, 0x105, 0x6B, 0x113` |
| `FUN_0060C930` | `[+0xDA]` | `0xBC, 0xBD, 0x102, 0xC2, 0xC9, 0xBC6` |
| `FUN_0060CCC0` | `[+0xDB]` | `0x103, 0xBC7` |
| `FUN_0060CDB0` | `[+0xDC]` (offset `piVar2[0x37]`) | `0x108, 0xBC6` |

Dialog `0xE2` (standard main menu) does **not** appear in any of those ID lists,
so for `0xE2` all four return `0` and write `0` into the corresponding record
bytes. These bytes gate whether optional Skirmish-preview/radar/minimap overlays
are drawn during `WM_PAINT_Handler`. For `0xE2`, all four flags are false and
those overlays are skipped — confirmed by the parent report's WM_PAINT analysis.

These are called once during `WM_INITDIALOG` via `FUN_00622B50`, not by
`ResizeShellChildControl_0060C0C0`. They are not invoked in the child-enumeration
resize pass.

---

## ResizeShellChildControl_0060C0C0 — Dispatcher

Address: `0x0060C0C0`  
Signature: `undefined4 ResizeShellChildControl_0060C0C0(HWND hwnd, HWND unused_param2)`  
Called via `EnumChildWindows(..., ResizeShellChildControl_0060C0C0, lparam)`.

### Guard: parent identity check

```
pHVar3 = GetParent(hwnd);
if (DAT_00AC48A8 != pHVar3) return 1;  // skip non-top-level children
```

`DAT_00AC48A8` is set to the dialog HWND by `FUN_0060C4A0` immediately before
calling `EnumChildWindows`. Any child whose `GetParent` does not match is skipped.

### Branch logic — which reposition helper is called

The dispatcher reads the dialog type id from the parent's dialog record
(`piVar1[0x1C]`), reads `GetDlgCtrlID(hwnd)` for the child control id, and routes
to one of eight move-math helpers. Every branch ends with `FUN_0060B950` (the
pixel-nudge finalizer) then `return 1`.

Full branch decision tree for dialog `0xE2` children:

1. **`FUN_00608500` returns non-zero** (specific dialog/control combos at specific
   screen widths with absolute hardcoded rects — e.g., Skirmish `0x94` / RA2 `0x103`
   / `0xBC`/`0xBD`/`0xC2`/`0xC9` modal variants, controls `0x6EA`, `0x6EC`,
   `0x50F`, `0x71E`, `0x670`, `0x7A7`, `0x7A8`, `0x72B`, `0x72F`, `0x732`, etc.)  
   → calls `FUN_0060AF50` (absolute-rect placer), then `FUN_0060B950`, return.  
   **For `0xE2` children**: `FUN_00608500` returns `0` (none of those dialog/control
   combos match), so this branch is not taken.

2. **`FUN_00608CD0` returns non-zero** (checks a button-state or dialog flag, not
   decompiled in scope)  
   → calls `FUN_0060B1D0` (right-anchored horizontal + vertical-delta mover), then
   `FUN_0060B950`, return.

3. **`FUN_00609730` returns non-zero** (another dialog flag check)  
   → calls `FUN_0060B350` (right-anchored button-row snap-to-grid placer), then
   `FUN_0060B950`, return.

4. **`FUN_00601360` returns non-zero AND control id == `0x695`**  
   → calls `FUN_0060B550` (bottom-left anchor), then `FUN_0060B950`, return.

5. **Parent dialog type id == `0xE2` AND control id == `0x71D`**  
   → calls `FUN_0060B610` (bottom-right anchor keyed to right-panel bottom cap),
   then `FUN_0060B950`, return.

6. **Parent dialog type id NOT in `{0xB7, 0x2B4, 0xBBB, 0xF5, 0x2B5, 0xB8, 0xA3,
   0xB6, 0x10C, 0x73, 0xFF, 0xEA}`**  
   → fallback: `MoveWindow(hwnd, window.left - parent.left, window.top - parent.top,
   window.w, window.h, 0)`, then `FUN_0060B950`, return.  
   (Re-expresses absolute screen rect as parent-relative without any
   scaling/offset — pure coordinate-space fixup.)

7. **Parent dialog type id IN that exclusion list**  
   → calls `FUN_0060B7A0` (widescreen absolute centering, only acts when
   `FUN_0069BBE0` returns non-zero), then `FUN_0060B950`, return.

**Active path for dialog `0xE2`** (corrected 2026-05-30 — see banner; `FUN_00608CD0` now
decompiled):
- **Branch 2** fires for the five non-Exit buttons `0x683/0x684/0x578/0x686/0x55C`
  (`FUN_00608CD0` true, kind 0) → `FUN_0060B000` (SDBTNANM-cell resize, 156×42 @ x=644),
  and for statics `0x694` (title) and `0x71C` (`FUN_00608CD0` true, not button-style) →
  `FUN_0060B1D0` (sidebar inset; title X=635).
- Branch 4 fires for child `0x695` (when `FUN_00601360` returns non-zero).
- Branch 5 fires for child `0x71D`.
- `0x71A` (movie) is repositioned outside `EnumChildWindows` by `FUN_0052B9B0`.
- Only Exit `0x3EE` (`FUN_00608CD0` false) reaches **branch 6**, the fallback coord-space
  fixup, keeping its raw template rect.

**Note on `FUN_00601360`:** Its internal logic was not decompiled in this pass.
The condition `FUN_00601360() != 0 && ctrl_id == 0x695` routes to `FUN_0060B550`.
Since `0x695` always needs to be bottom-left anchored whenever the dialog is
fullscreen-expanded, the reasonable inference is that `FUN_00601360` returns
non-zero for `0xE2` during the fullscreen-expand pass. Confidence: medium (inferred
from caller structure; not decompiled).

---

## Move-Math Helpers — Per-Function Analysis

### FUN_0060AF50 — Absolute-rect placer (branch 1)

Address: `0x0060AF50`  
Signature: `void FUN_0060AF50(HWND hwnd, int *rect_ptr)`  
Active in YR: Yes, for specific dialog/control combos. **NOT reached for `0xE2`.**

`rect_ptr` points to an int[4] `{x, y, w, h}` allocated by `FUN_00608500`.

Widescreen offset variables:
```
delta_x = 0; delta_y = 0;
if (!FUN_0069BBE0()) {   // if NOT small/fullscreen mode
    delta_x = max(0, (parent_w - 800) / 2);
    delta_y = max(0, (parent_h - 600) / 2);
}
MoveWindow(hwnd, rect_ptr[0] + delta_x, rect_ptr[1] + delta_y, rect_ptr[2], rect_ptr[3], 0);
InvalidateRect(hwnd, NULL, 0);
```

Math: adds a centering offset to a hardcoded `{x,y,w,h}` rect. The centering
offset is `(parent_dim - base_dim) / 2`, clamped to `≥ 0`, where base dims are
`DAT_007F5BE4 = 800` and `DAT_007F5BF0 = 600`.

`FUN_0069BBE0` reads `*(byte*)(param_1 + 0x30D8)` — a flag byte in what appears
to be a per-dialog record. When non-zero it indicates the dialog is in "small" or
alternate mode; when zero (the normal YR shell), the centering offsets apply.

---

### FUN_0060B000 — Right-anchored PCX-button absolute placer with grid snap

Address: `0x0060B000`  
Signature: `void FUN_0060B000(HWND hwnd, int param_2)`  
Active in YR: Yes — Skirmish/multiplayer button controls **and the five non-Exit `0xE2`
main-menu buttons** (`0x683/0x684/0x578/0x686/0x55C`). **Reached for `0xE2`** (corrected
2026-05-30, see banner): sizes the button window to the `SDBTNANM.SHP` cell (156×42) at
x=644, grid-snapping Y to the button-column rows.

Places shell buttons right-of-center using `g_SDBTNANM_SHP` frame dimensions
for height. Reads `*(short*)(g_SDBTNANM_SHP + 2)` as width and
`*(short*)(g_SDBTNANM_SHP + 4)` as height, X = `(parent_right - delta_x) - 0x9C`.
In the `cVar3 != '\0'` (small) branch, uses `DAT_00B0F9EC` dims and
`X = parent_w - 0x93`. Snaps Y to the nearest grid row using
`DAT_00B0FC24 + 0xC` as row height and `DAT_00B0FC24 + 4` as top-anchor.

---

### FUN_0060B1D0 — Right-anchored horizontal + vertical-delta mover

Address: `0x0060B1D0`  
Signature: `void FUN_0060B1D0(HWND hwnd, int param_2)`  
Active in YR: Yes. **Reached for `0xE2`** statics `0x694` (title) and `0x71C` — corrected
2026-05-30: `FUN_00608CD0` DOES return non-zero for `0xE2` (`decompile_function
0x00608CD0`). The title's final X is therefore 635 (sidebar inset `(168-w)/2 = 3`), not
the raw 638 this report's body implies. The five `0xE2` buttons are kind 0 and take
`FUN_0060B000` instead.

Math (normal shell, `FUN_0069BBE0 == 0`):
```
delta_x = max(0, (parent_w - 800) / 2)
v_top_delta = max(0, (parent_h - 600) / 2)
v_screen_delta = max(0, (param_2.h - 600) / 2)
inset = (g_SIDEBAR_WIDTH_CONST - ctrl_w) / 2     // if no override: (168 - w) / 2
X = (parent_right - inset - ctrl_w - delta_x) - parent_left
Y = (ctrl_top - parent_top) + (v_top_delta - v_screen_delta)
MoveWindow(hwnd, X, Y, ctrl_w, ctrl_h, 0)
```

Right-anchors the control to the right edge minus sidebar inset, while adjusting Y
by the difference between parent's top margin and a "screen-relative" top margin
derived from `param_2`.

---

### FUN_0060B350 — Right-anchored button snap-to-grid placer

Address: `0x0060B350`  
Signature: `void FUN_0060B350(HWND hwnd)`  
Active in YR: Yes (for Skirmish dialog controls whose parent `FUN_00609730` is set). 
**NOT reached for `0xE2`.**

Math (normal shell):
```
delta_x = max(0, (parent_w - 800) / 2)
X = (parent_right - delta_x) - parent_left - 0x9C
Y = floor((DAT_00B0FC28.y - DAT_00B0FC24.y) / DAT_00B0FC24.row_h - 1)
    * DAT_00B0FC24.row_h + DAT_00B0FC24.y
```
Height and width from `*(short*)(g_SDBTNANM_SHP + 4/2)`.
Small-mode branch: `X = parent_w - 0x93`, `Y = DAT_00B0FC4C.y - h`.

---

### FUN_0060B420 — Right-anchored top-of-bottom-cap mover

Address: `0x0060B420`  
Signature: `void FUN_0060B420(HWND hwnd)`  
Active in YR: Yes (for controls in dialogs handled by `FUN_00609730`). **NOT
reached for `0xE2`.**

Math:
```
delta_x = max(0, (parent_client_w - 800) / 2)
inset = (g_SIDEBAR_WIDTH_CONST - ctrl_w) / 2   // or override from record[0x38]
X = (parent_client_right - inset - ctrl_w - delta_x) - parent_client_left
Y = (normal: DAT_00B0FC28.y) or (small: DAT_00B0FC50.y) - ctrl_h
```

---

### FUN_0060B550 — Bottom-left anchor (0x695 tooltip/status)

Address: `0x0060B550`  
Signature: `void FUN_0060B550(HWND hwnd)`  
**Called for control `0x695` on dialog `0xE2`. Active in YR: Yes.**
Only caller: `ResizeShellChildControl_0060C0C0`.

Math (normal shell, `FUN_0069BBE0 == 0`):
```
hWnd_parent = GetParent(hwnd)
GetClientRect(parent, &parent_rect)      // client coordinates
GetWindowRect(hwnd, &ctrl_rect)          // screen coordinates
delta_x = max(0, (parent_client_w - 800) / 2)
delta_y = max(0, (parent_client_h - 600) / 2)
X = delta_x + 10 + parent_rect.left
Y = (parent_client_h - ctrl_h - delta_y) - 1
MoveWindow(hwnd, X, Y, ctrl_w, ctrl_h, 0)
```

In the small/alternate mode (`FUN_0069BBE0 != 0`): `delta_x = 0`, `delta_y = 0`,
so `X = 10`, `Y = parent_h - ctrl_h - 1`.

**Summary for `0x695`:**
- X is anchored `10 px` left of the shell's left centering margin.
  At `800×600`: `X = 0 + 10 = 10`. At `1024×768`: `X = 112 + 10 = 122`.
- Y is anchored `1 px` above the client bottom, adjusted upward by the vertical
  centering margin.
  At `800×600`: `Y = 600 - ctrl_h - 0 - 1 = 599 - ctrl_h`.
  At `1024×768`: `Y = 768 - ctrl_h - 84 - 1 = 683 - ctrl_h`.
- Size is preserved (MoveWindow keeps `ctrl_w, ctrl_h`).

---

### FUN_0060B610 — Bottom-right anchor keyed to right-panel bottom cap (0x71D)

Address: `0x0060B610`  
Signature: `void FUN_0060B610(HWND hwnd)`  
**Called for control `0x71D` on dialog `0xE2`. Active in YR: Yes.**
Only caller: `ResizeShellChildControl_0060C0C0`.

Math (normal shell, `FUN_0069BBE0 == 0`):
```
hWnd_parent = GetParent(hwnd)
GetClientRect(parent, &parent_rect)      // client coordinates
GetWindowRect(hwnd, &ctrl_rect)          // screen coordinates
delta_x = max(0, (parent_client_w - 800) / 2)
ctrl_w = ctrl_rect.right - ctrl_rect.left

// inset: use record[0x38] override if set, else default
inset = (g_SIDEBAR_WIDTH_CONST - ctrl_w) / 2    // default: (168 - ctrl_w) / 2

// Y anchor: bottom of right-panel normal-mode rect
Y_anchor = DAT_00B0FC28.y + DAT_00B0FC28.h      // i.e., *(int*)(DAT_00B0FC28+0xC) + *(int*)(DAT_00B0FC28+4)

X = (parent_client_right - inset - ctrl_w - delta_x) - parent_client_left
Y = Y_anchor - ctrl_h
MoveWindow(hwnd, X, Y, ctrl_w, ctrl_h, 0)
```

Small mode (`FUN_0069BBE0 != 0`) uses `DAT_00B0FC50` instead of `DAT_00B0FC28`
for the Y anchor.

**Summary for `0x71D`:**
- X is right-anchored inside the sidebar area. At `800×600` with `ctrl_w ≈ 162`:
  `inset = (168 - 162) / 2 = 3`,
  `X = (800 - 3 - 162 - 0) - 0 = 635`.
  (Note: the raw DLU-derived x is `638`; the sidebar-inset formula brings it to
  `635`.)
- Y is the bottom edge of the right-panel lower-cap rect minus `ctrl_h`.
  This ties the version line's bottom edge to the bottom edge of the SDBTM cap.
- Size is preserved.

`DAT_00B0FC28` is the normal-shell right-panel lower-cap rect struct. Its layout:
offset `+4` = top y, offset `+0xC` = height.
`DAT_00B0FC50` is the small-shell equivalent.

---

### FUN_0060B7A0 — Widescreen absolute centering (button-row dialogs)

Address: `0x0060B7A0`  
Signature: `void FUN_0060B7A0(HWND hwnd)`  
Active in YR: Only when `FUN_0069BBE0 != 0` (small/fullscreen alternate mode).  
**NOT reached for `0xE2`** (dialog `0xE2` is not in the exclusion list that routes
to this helper; `0xE2` goes to the fallback coord-space fixup instead).

When called and `FUN_0069BBE0 != 0`:
```
GetWindowRect(hwnd, &ctrl_rect)
GetWindowRect(parent, &parent_rect)
parent_w = parent_rect.right - parent_rect.left
parent_h = parent_rect.bottom - parent_rect.top
delta_x = 0; delta_y = 0;
if (parent_w > 800):  delta_x = max(0, (parent_w - 800) / 2)
elif (parent_w < 800): delta_x = (parent_w - 800) / 2   // negative
if (parent_h > 600):  delta_y = max(0, (parent_h - 600) / 2)
elif (parent_h < 600): delta_y = (parent_h - 600) / 2   // negative
new_x = ctrl_rect.left + delta_x
new_y = ctrl_rect.top + delta_y
if new_x < 0: new_x = 0
if new_y < 0: new_y = 0
MoveWindow(hwnd, new_x, new_y, ctrl_w, ctrl_h, 0)
```

Translates the control's absolute screen rect by the widescreen delta, clamping
to zero. Only fires in the small/fullscreen-alternate shell mode.

---

### FUN_0060B950 — Pixel-nudge finalizer (0x694 heading nudge path)

Address: `0x0060B950`  
Signature: `void FUN_0060B950(HWND hwnd)`  
Called at the end of every branch in `ResizeShellChildControl_0060C0C0`.  
**Active in YR: Yes for all children, including `0x694`.**

This function applies optional sub-pixel nudges based on the parent dialog type id
(`parent_record[0x1C]`) and the child control id. For dialog `0xE2` and child
`0x694` specifically:

```
iVar10 = parent_record[0x1C]   // = 0xE2
iVar5  = GetDlgCtrlID(hwnd)    // = 0x694
```

The condition tree checks if `iVar5 == 0x694` and `iVar10` is in an extended set
of dialog IDs that includes `0xE2`. It then goes to `LAB_0060BC9F`, which checks
whether `iVar10` is in `{0xBC, 0xBD, 0x102, 0xC2, 0xC9, 0xBC6, 0x105, 0x6B,
0x113}`. `0xE2` is NOT in that inner set, so it falls to the outer sub-tree.

In the outer sub-tree, `0xE2` is not in the deeper exclusion chains, so the path
reaches:

```
cVar2 = FUN_0069BBE0()
if (cVar2 != '\0'): return   // small mode: no nudge
iVar9 = iVar8 + 7            // Y nudge: top moves down by +7 px
local_34 = iVar3 + 1         // height nudge: height grows by +1 px
```

Then at `LAB_0060C092`: `MoveWindow(hwnd, local_3c, iVar9, local_38, local_34, 0)`.

**Summary for `0x694` in `0xE2`:**
- Y += 7 (top moves 7 px down from the position set by the earlier branch).
- height += 1.
- X and width unchanged.
- In small/fullscreen-alternate mode (`FUN_0069BBE0 != 0`): no nudge at all,
  `FUN_0060B950` returns immediately.

For all other `0xE2` children, `FUN_0060B950` typically returns without a
`MoveWindow` call because the control id (`0x695`, `0x71D`, buttons, etc.) does
not match the `0x694` branch, and the parent id `0xE2` does not trigger any of
the other specific cases (most of which target Skirmish/multiplayer dialog IDs).

---

## Caller Chain

### Full dispatch chain to `ResizeShellChildControl_0060C0C0`

```
FUN_00531CC0 (YR main menu entry point)
  └─ FUN_00622650(0)         creates dialog 0xE2
  └─ CenterChildWindow()
  └─ FUN_00622800()          ShowWindow + SetForeground
  └─ (dialog WM_INITDIALOG handled by FUN_00622B50)
       └─ FUN_0060C540()
            └─ FUN_0060C4A0(hwnd, lparam)
                 MoveWindow(hwnd, 0, 0, g_ScreenWidth, g_ScreenHeight, 0)
                 DAT_00AC48A8 = hwnd
                 EnumChildWindows(hwnd, ResizeShellChildControl_0060C0C0, lparam)
```

Alternate entry:
```
FUN_00622820(hwnd, dialog_type)   called by FUN_00648710, FUN_0079FC40, etc.
  └─ FUN_0060C540() → FUN_0060C4A0 → EnumChildWindows(..., ResizeShellChildControl_0060C0C0, ...)
     (only when FUN_0060C540 returns non-zero, i.e., for "expanded" dialog types)
```

`FUN_00622820` is also called on `WM_INITDIALOG` re-entries and dialog mode
transitions; it re-runs the full resize pass.

### `FUN_0052B9B0` — separate 0x71A movie repositioner

`FUN_0052B9B0` is NOT in the `EnumChildWindows` resize path. It is a standalone
function that repositions only `0x71A`. Called directly by the dialog setup code
(and `FUN_00531CC0` calls the equivalent inline logic):

```
hWnd = GetDlgItem(parent, 0x71A)
X = (screen_w < 801) ? 0 : (screen_w - 800) / 2
Y = (screen_h < 601) ? 0 : (screen_h - 600) / 2
SetWindowPos(hWnd, NULL, X, Y, -1, -1, 0x0D)   // SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE
SendMessage(hWnd, 0x4E3, 1, 0)                  // set loop flag
SendMessage(hWnd, 0x4E4, 0, screen_w == 640 ? "Ra2ts_s" : "Ra2ts_l")
```

This is the ONLY reposition applied to `0x71A`. It uses `< 801` / `< 601`
thresholds (not `<= 800` / `<= 600`), meaning at exactly `800×600` x=0, y=0.

---

## Control-to-Helper Mapping Summary (dialog `0xE2` only)

| Control | Helper(s) called | Math summary |
|---:|---|---|
| `0x71A` (movie) | `FUN_0052B9B0` (direct, outside EnumChildWindows) | `X=max(0,(w-800)/2)`, `Y=max(0,(h-600)/2)` |
| `0x695` (tooltip) | `FUN_0060B550` → `FUN_0060B950` | Left+10 from centering margin, bottom-1 from client bottom |
| `0x71D` (version) | `FUN_0060B610` → `FUN_0060B950` | Right-inset of sidebar, Y = right-panel-bottom-cap-bottom - ctrl_h |
| `0x694` (heading) | `FUN_0060B1D0` → `FUN_0060B950` | Sidebar inset (X=635), then Y+7, H+1 nudge in `FUN_0060B950` (corrected 2026-05-30) |
| buttons `0x683/0x684/0x578/0x686/0x55C` | `FUN_0060B000` → `FUN_0060B950` | Resize to SDBTNANM cell 156×42, X=644 (flush right), grid-snap Y (corrected 2026-05-30) |
| Exit button `0x3EE` | fallback coord-fixup → `FUN_0060B950` | Raw template rect (≈162×37 @ x=638, y=536); `FUN_00608CD0` false so NOT resized |
| `0x71C` (static) | `FUN_0060B1D0` → `FUN_0060B950` | Sidebar inset; no visible output in `0xE2` |

---

## Static Constants Referenced

| Address | Ghidra label | Value | Role in helpers |
|---:|---|---:|---|
| `0x007F5BE0` | `DAT_007F5BE0` | `640` | small shell width |
| `0x007F5BE4` | `DAT_007F5BE4` | `800` | standard shell width |
| `0x007F5BE8` | `DAT_007F5BE8` | `1024` | high-res peer |
| `0x007F5BEC` | `DAT_007F5BEC` | `480` | small shell height |
| `0x007F5BF0` | `DAT_007F5BF0` | `600` | standard shell height |
| `0x007F5BF8` | `g_SIDEBAR_WIDTH_CONST` | `168` | sidebar width for X-inset in `FUN_0060B610`, `FUN_0060B1D0`, `FUN_0060B420` |
| `0x00AC48A8` | `DAT_00AC48A8` | HWND | current dialog HWND set before EnumChildWindows |
| `0x00B0FC28` | `DAT_00B0FC28` | struct ptr | normal-shell right-panel lower-cap rect (`+4`=top_y, `+0xC`=height) |
| `0x00B0FC50` | `DAT_00B0FC50` | struct ptr | small-shell right-panel lower-cap rect equivalent |

`g_SIDEBAR_WIDTH_CONST = 168` is the same value as `RIGHT_PANEL_WIDTH` already in
`src/ui/main_menu_shell/layout.rs`. The X-inset formula for `0x71D` is
`(168 - ctrl_w) / 2`, which at `ctrl_w = 162` gives `inset = 3`, so `X ≈ 635`
not `638` at `800×600`.

---

## Gaps and Open Questions

1. **`FUN_00601360`** — the predicate gating the `0x695` → `FUN_0060B550` branch —
   was not decompiled. Assumption: it returns non-zero for dialog `0xE2` (all
   evidence points that way), but not verified from the body. Confidence: medium.

2. **`DAT_00B0FC28` exact struct layout** — confirmed `+4` = top_y and `+0xC` =
   height from the `FUN_0060B610` and `FUN_0060B350` decompilations, but the full
   struct (especially the base address of the right-panel lower-cap rect) was not
   re-verified in this session against the right-panel layout code.

3. **FUN_0069BBE0 threshold** — reads `*(byte*)(param_1 + 0x30D8)`. The specific
   condition that sets this byte to non-zero (triggering "small mode" behavior) was
   not traced. For standard `800×600` and `1024×768` YR the byte is `0` (normal
   shell mode applies).

---

## Verification Summary

| Claim | Evidence | Confidence |
|---|---|---|
| `FUN_0060CAF0/C930/CCC0/CDB0` are classifiers, not resize helpers | Live decompilation: all four bodies are hash-lookup + flag-write patterns, no MoveWindow | Verified |
| `FUN_0060B550` handles `0x695`, anchors bottom-left | Live decompilation of `FUN_0060B550`; caller confirmed via `get_function_callers` | Verified |
| `FUN_0060B610` handles `0x71D`, anchors to right-panel bottom cap | Live decompilation of `FUN_0060B610`; check `iVar5 == 0xE2 && iVar6 == 0x71D` in dispatcher | Verified |
| `FUN_0060B950` applies Y+7, H+1 to `0x694` in dialog `0xE2` | Live decompilation of `FUN_0060B950`; branch confirmed for `iVar10==0xE2, iVar5==0x694` | Verified |
| `FUN_0052B9B0` is the sole repositioner of `0x71A`, outside EnumChildWindows | Live decompilation; threshold `< 801` / `< 601` | Verified |
| `g_SIDEBAR_WIDTH_CONST = 168` | Ghidra label on `0x007F5BF8`, used in `FUN_0060B610` for X inset | Verified |
| `EnumChildWindows` path enters via `FUN_0060C4A0`, called from `FUN_00622820` | Live decompilation of both; xref of `ResizeShellChildControl_0060C0C0` = `0x00622A62` and `0x0060C4C7` | Verified |
