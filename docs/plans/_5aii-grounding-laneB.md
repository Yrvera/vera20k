# Slice 5a-ii Grounding — Lane B (Binary Verification)

Read-only Ghidra verification of the in-game Options dialog (RT_DIALOG template **0xBBB**)
owner-draw paint path, SHP canvas-size anchoring, child-resize/reposition helper family, and
absence of a full-screen image control. Authority: binary -> Ghidra -> docs. Every claim below
cites the decompile/disassemble/read call that produced it (in this session, gamemd.exe).

Image base = 0x00400000 (addresses below are Ghidra/static addresses).

---

## Executive verdict per task item

| # | Item | Verdict | Key correction |
|---|------|---------|----------------|
| 1 | Owner-draw button paint routing (0x686/0x52C/0x52D) | **verified** | Paint is NOT in the Options proc nor the WM_DRAWITEM handler; it is in `OwnerDraw_Button_00612B70`. Frames are **2/3/4** (SDBTNANM skin) or 0/1/2 (other skins), NOT a uniform 0/1/2. |
| 2 | "SIDEBTTN.SHP canvas-size read at struct offset 0x93" | **WRONG as framed** | 0x93 is a **literal pixel inset constant** in the X-anchor formula, not a struct offset. Canvas width is read from the **SHP header at +2/+4**. |
| 3 | Reposition family 0060B000/0060B350/0060B1D0/0060B7A0/0060B950 | **verified** | All exist, reachable for 0xBBB in normal YR skirmish via `ResizeShellChildControl_0060C0C0` (stretch path). NOT TS-legacy. |
| 4 | No full-screen image control created (overlay-only) | **verified** | Init helpers only set per-control flags; no background-image surface allocated. |

---

## (1) Owner-draw button paint path

### Options proc dispatch — verified via `decompile_function 0x004E1FE0`
`FUN_004e1fe0(hDlg, msg, wParam, lParam)` first calls the shared shell-dialog framework
`FUN_00622b50(msg, lParam)` (`iVar1 = FUN_00622b50(param_3,param_4)`); if that returns nonzero the
proc returns early. The proc itself handles only WM_COMMAND (0x111) for 0x52C (Keyboard ->
g_GameState=4), 0x52D (Sound -> g_GameState=6), 0x686 (Back -> consume), WM_HSCROLL (0x114) for the
three trackbar value-labels (0x671/0x672/0x673, plus 0x670 in non-active), and the populate path
(0x497). **The Options proc does not paint the owner-draw buttons.**

### Where WM_DRAWITEM (0x2B) goes — verified via `decompile_function 0x00622B50`
The shared framework `FUN_00622b50` switch routes `case 0x2b:` -> `FUN_006213a0()` then `return 1`.
So WM_DRAWITEM is intercepted by the framework, never reaching `FUN_004e1fe0`.

### What the WM_DRAWITEM handler does — verified via `decompile_function 0x006213A0`
`FUN_006213a0(int *drawitem)` treats `drawitem` as a DRAWITEMSTRUCT-like record:
- `drawitem[0] == 4` checks CtlType == ODT_BUTTON.
- `drawitem[5]` = hwndItem (the button HWND).
- It looks the HWND up in the shell control-record hashmap (`DAT_00ac1b00`), then writes
  `puVar2[0x3b] = drawitem[4]` (the itemState into control-record offset **0xEC**), then
  `InvalidateRect(hWnd, drawitem+7, 0)` + `UpdateWindow(hWnd)`.

So the handler **does not draw SHP frames itself** — it records the button's draw state and forces the
button's own subclass WndProc to repaint. This is the correct "internal mechanism": the paint is
deferred to the per-button owner-draw proc on WM_PAINT.

### How a button HWND acquires its owner-draw proc — verified via `decompile_function 0x0060F9A0`
`FUN_0060f9a0(hWnd, param)` (an EnumChildWindows callback used at WM_INITDIALOG) calls
`GetClassNameA` and string-matches the class. For class "Button", it reads the window style
(`GetWindowLongA(hWnd,-0x10)` = GWL_STYLE) low byte and picks the WndProc:
- `(style & 7) == 7`  -> `OwnerDraw_ButtonVariant_0061E700` (tag 0x0)
- `(style & 0xB) == 0xB` -> `OwnerDraw_Button_00612B70` (tag 0x0)  <-- **the BS_OWNERDRAW sidebar button**
- `(style & 3) == 3`  -> `OwnerDraw_Checkbox_006163A0`
- `(style & 9) == 9`  -> `OwnerDraw_RadioVariant_00616980`

The three 0xBBB owner-draw buttons (Back 0x686, Keyboard 0x52C, Sound 0x52D) are BS_OWNERDRAW and
take the `OwnerDraw_Button_00612B70` arm. It then `SetWindowLongA(hWnd,-4, 0x610ca0)` to install the
shared dispatch thunk and stores the original WndProc + the picked owner-draw proc in two hashmaps
(`DAT_00ac18c0`, `DAT_00ac1b48`).

### The actual SHP frame selection — verified via `decompile_function 0x00612B70`
`OwnerDraw_Button_00612B70(hWnd,msg,wParam,lParam)` on WM_PAINT (case 0xF) reads the control record
and selects the graphic by a **skin-mode selector** `piVar17[0x2c]` (control-record offset **0xB0**):

- **mode 0** (PCX cap path): builds `b_c_c_li_*.pcx` / `mi` / `ri` (left/middle/right cap, prefix
  `'u'`/`'d'` chosen from the pressed flag at `0xC5`), stitched horizontally. No SHP. (strings
  `s_b_c_c_li_d_pcx_0083589c`, `_mi_` 0083588c, `_ri_` 0083587c.)
- **mode 1** (`iVar14 == 1`): SHP = `FUN_0072e2c0()` -> `g_SDBTNANM_SHP`-class global `DAT_00b0fbdc`;
  palette `g_SDBTNANM_SHP` (`piStack_dc`). Frame `local_f0`:
  - released = **0x2**
  - pressed (radio/down bit `pWStack_d8 & 1`) = **0x4**
  - hover/checked (record `0xC5` set) = **0x3**
- **mode 2** (`iVar14 == 2`): SHP = `FUN_0072f4b0()` -> `DAT_00b0fbe8`; palette `DAT_00b0f9ec`.
  Frame: down = 0x1, checked = 0x2, else 0x0.
- **mode 3** (`iVar14 == 3`): SHP = `FUN_0072b050()` -> `DAT_00b0fb78`; palette `DAT_00b0facc`.
  Frame: down = 0x1, checked = 0x2, else 0x0.

Then `CC_Draw_Shape(palette, frame, &hwndItem, &rect, 0x400, ...)` blits the chosen frame.

**Correction to the brief:** the released/hover/pressed -> frame mapping is **not** a fixed 0/1/2.
For the SDBTNANM skin (mode 1) it is **2 (released) / 3 (hover) / 4 (pressed)**; for the other SHP
skins (modes 2/3) it is **0 (released) / 2 (checked) / 1 (pressed)**. Which skin a given 0xBBB button
uses is determined at runtime by record-offset 0xB0, set during init (see `FUN_0060aab0`/`FUN_00622820`
attribute passes). Confidence: **verified** for the frame tables and SHP-global routing; the precise
per-button 0xB0 value for Back/Keyboard/Sound is set by data-driven init not transcribed here
(UNCHECKED which of mode 1/2/3 each lands in — but all three modes are SHP-frame paths, none paint
via the dialog proc).

`SIDEBTTN.SHP` vs `SDBTNANM.SHP`: both are entries in the shell filename pointer-table around
0x00844cd4 (verified via `read_memory 0x00844cf0` / `0x00844cf8`; `get_xrefs_to 0x008450f4` -> DATA
@0x00844cfc; `get_xrefs_to 0x00845178` -> DATA @0x00844cd4). They load into sibling shell-graphic
globals consumed by the button/anchor code; SIDEBTTN is the released-state base, SDBTNANM the animated
overlay frames.

---

## (2) SHP canvas-size read and the "struct offset 0x93" claim

### What 0x93 / 0x9c actually are — verified via `decompile_function 0x0060B000`
`FUN_0060b000(hWnd, pParam)` (the right-edge button anchoring helper) computes:

```
cVar3 = FUN_0069bbe0(dialogRecord)        // returns *(u8*)(rec + 0x30d8) -- a "stretch/skin" flag
...
if (cVar3 == 0) {                          // SDBTNANM animated path
    width  = *(short*)(g_SDBTNANM_SHP + 2)   // SHP header canvas WIDTH
    height = *(short*)(g_SDBTNANM_SHP + 4)   // SHP header canvas HEIGHT
    X = ((parentRight - centeredOffset) - parentLeft) + -0x9c     // 0x9C = 156 px right inset
    ... vertical = nearest multiple of frame-pitch DAT_00b0fc24+0xc ...
} else {                                   // alternate (DAT_00b0f9ec) path
    width  = *(short*)(DAT_00b0f9ec + 2)
    height = *(short*)(DAT_00b0f9ec + 4)
    X = ((parentRight - centeredOffset) - parentLeft) + -0x93     // 0x93 = 147 px right inset
    ... vertical from (((bottom-top)/2 - 0xc6 + (top)) / 0x2c) * height ...
}
MoveWindow(hWnd, X, Y, width, height, 0)
```

So:
- **0x93 (147) and 0x9c (156) are literal pixel right-edge inset constants**, immediates in the X
  formula, selected by the `FUN_0069bbe0` flag. They are **not** a struct offset and are **not** a
  canvas-size read.
- The **canvas width/height** the anchoring depends on is read from the **SHP file header at +2
  (width) and +4 (height)** of whichever shell-graphic global is active (`g_SDBTNANM_SHP` or
  `DAT_00b0f9ec`). Verified via `decompile_function 0x0060B000`.

`FUN_0069bbe0` confirmed via `decompile_function 0x0069BBE0`: it is a one-liner
`return *(u8*)(param_1 + 0x30d8)` — a dialog-record flag, unrelated to canvas size.

**Verdict: the brief's "canvas-size read at struct offset 0x93" is WRONG/mislabeled.** The real
dependency is (a) the SHP-header width at +2 of the active sidebar-button SHP, and (b) the literal
right-inset 0x93/0x9c. The Rust port's x-anchoring must read the SHP header width and apply the
147/156-px inset (whichever branch the active skin flag selects), not look up a struct field 0x93.
Confidence: **verified** (single caller of `FUN_0060b000` is `ResizeShellChildControl_0060C0C0`,
confirmed via `get_function_callers 0x0060B000` and `get_xrefs_to 0x0060B000`).

---

## (3) Child-resize/reposition helper family

### Existence — verified via `get_function_by_address`
- `FUN_0060b000` @0060b000 (body 0060b000-0060b1ce) — right-edge SHP-anchored button reposition.
- `FUN_0060b1d0` @0060b1d0 (0060b1d0-0060b341).
- `FUN_0060b350` @0060b350 (0060b350-0060b41d).
- `FUN_0060b7a0` @0060b7a0 (0060b7a0-0060b878) — **centered-offset reposition** (the 0xBBB arm).
- `FUN_0060b950` @0060b950 (0060b950-0060c0b6) — common finalizer, called after every branch.

### The dispatcher — verified via `decompile_function 0x0060C0C0`
`ResizeShellChildControl_0060C0C0(hChild, lParam)` is the per-child router (passed to
EnumChildWindows). It guards `GetParent(hChild) == DAT_00ac48a8` then routes by class/style/tag:
- owner-draw button (`style & 0xB == 0xB`, gated `FUN_00608cd0`) -> `FUN_0060b000` -> `FUN_0060b950`
- `FUN_00608cd0` true -> `FUN_0060b1d0` -> `FUN_0060b950`
- owner-draw + record-offset 0x68==0, `FUN_00609730` true -> `FUN_0060b350` -> `FUN_0060b950`
- `FUN_00609730` true -> `FUN_0060b420` -> `FUN_0060b950`
- footer static id 0x695 + `FUN_00601360` -> `FUN_0060b550`
- the **template-tag branch**: reads control-record `piVar1[0x1c]` (the populated template id at
  offset **0x70**); if it is NOT one of {0xb7,0x2b4,**0xbbb**,0xf5,0x2b5,0xb8,0xa3,0xb6,0x10c,0x73,
  0xff,0xea}, do a plain "preserve absolute position" MoveWindow; **otherwise -> `FUN_0060b7a0`**.

Because **0xBBB is in that set**, the in-game Options dialog children route through
`FUN_0060b7a0` -> `FUN_0060b950`.

### Centered-offset (0060B7A0) vs right-edge (0060B000) anchoring
- **`FUN_0060b7a0` (verified via `decompile_function 0x0060B7A0`) = centered offset.** Gated by
  `FUN_0069bbe0`. It computes `dx = (parentWidth - DAT_007f5be4)/2` and
  `dy = (parentHeight - DAT_007f5bf0)/2` (sign-corrected toward 0), where `DAT_007f5be4 / 007f5bf0`
  are the design (base) dialog width/height. New top-left = `(childLeft+dx, childTop+dy)`, clamped to
  >= 0, same size. This recenters every child by the same delta when the dialog is stretched to the
  current resolution. **This is the path the 0xBBB Options children take.**
- **`FUN_0060b000` (verified above) = right-edge button anchoring.** Pins a button to the parent's
  right edge minus a fixed 147/156-px inset, sized to the SHP canvas, vertically snapped to the
  animated frame pitch. Used for the BS_OWNERDRAW sidebar-style buttons.
- **`FUN_0060b950` (verified via `decompile_function 0x0060B950`) = finalizer / per-(template-id,
  control-id) nudge.** A large switch on `(piVar1[0x1c]==templateId, GetDlgCtrlID(child))` applying
  +/-1..+10 px tweaks; for template **0xbbb** it is in the recognized set and applies the
  `iVar5 == 0x694` footer/title nudge family (mostly +1/+2 px and a `FUN_0069bbe0`-gated +7 path).

### Reachability in a normal YR skirmish — NOT TS-legacy — verified
- Single caller of each helper is `ResizeShellChildControl_0060C0C0`
  (`get_function_callers 0x0060B000/0x0060B350/0x0060B7A0`).
- `ResizeShellChildControl_0060C0C0` is referenced (as an EnumChildWindows callback) from
  `FUN_00622820` @00622a62 and `FUN_0060c4a0` @0060c4c7 (`get_xrefs_to 0x0060C0C0`, both DATA).
- `FUN_0060c4a0` (verified via `decompile_function 0x0060C4A0`):
  `MoveWindow(dlg,0,0,g_ScreenWidth,g_ScreenHeight); EnumChildWindows(dlg,ResizeShellChildControl,..)`.
- `FUN_0060c4a0` is invoked from the shell-dialog WM_INITDIALOG path in `FUN_00622b50` when
  `FUN_0060c540()` returns true (verified in the 0x110 arm of `FUN_00622b50`).
- `FUN_0060c540` (verified via `decompile_function 0x0060C540`) returns TRUE exactly when the
  control-record template id `piVar3[0x1c]` is in its recognized set — **0xBBB is explicitly in that
  set.** So for the Options dialog, the stretch+resize path fires.
- `FUN_00622820` (verified via `decompile_function 0x00622820`) is the alternate WM_INITDIALOG body
  and likewise calls `EnumChildWindows(dlg, ResizeShellChildControl_0060C0C0, &local_8)` when
  `FUN_0060c540()` is true.

This is a live, default YR shell-dialog stretch/reposition pipeline (it keys off the current
`g_ScreenWidth/g_ScreenHeight` vs the base design size `DAT_007f5be4/007f5bf0`). No SpecialFlags gate,
no TS-only branch. Confidence: **verified**.

---

## (4) No full-screen image control created (overlay-only)

The WM_INITDIALOG path (`FUN_00622b50` 0x110 arm) calls a chain of init helpers before/after the
EnumChildWindows passes: `FUN_0060caf0`, `FUN_0060c930`, `FUN_0060ccc0`, `FUN_0060cdb0`,
`FUN_0060cf00`, plus the subclass installer `FUN_0060f9a0` and attribute setter `FUN_0060aab0`.

- `FUN_0060caf0` (verified via `decompile_function 0x0060CAF0`): only sets per-control byte flag at
  record+0xD9 based on template id. No surface/image creation.
- `FUN_0060c930` (verified via `decompile_function 0x0060C930`): only sets record+0xDA flag. No image.
- `FUN_0060aab0` (verified via `decompile_function 0x0060AAB0`, function boundary created this
  session for navigation): sets record fields +0xDC (scroll step) and +0xC8 (padding/alignment) from
  class/template id. No image-control creation.
- The only blits are the per-button owner-draw SHP/PCX paints in `OwnerDraw_Button_00612B70` and the
  framework's background-fill via `GetStockObject(4)` for WM_CTLCOLOR* (`FUN_00622b50`, the
  0x132..0x138 arm returns `GetStockObject(4)`).

No code path in the dialog init allocates a full-screen background-image surface or creates a static
image control sized to the dialog/screen. The dialog draws its chrome per-control over whatever is
behind it (the in-game sidebar/tactical view), i.e. **overlay-only**. Confidence: **verified** for
the inspected init chain; the chain is the standard shell-dialog init shared by all 0xBBB-class
dialogs.

---

## Address ledger (this session)

| Address | Role | Verified by |
|---------|------|-------------|
| 0x004E1FE0 | Options dialog proc (0xBBB) | decompile_function |
| 0x00622B50 | shared shell-dialog framework; WM_DRAWITEM->0x006213A0; WM_INITDIALOG->resize | decompile_function |
| 0x006213A0 | WM_DRAWITEM handler: records itemState @rec+0xEC, invalidates button | decompile_function |
| 0x0060F9A0 | owner-draw subclass installer (class/style -> WndProc) | decompile_function |
| 0x00612B70 | OwnerDraw_Button: SHP/PCX paint, frame tables | decompile_function |
| 0x0061E700 | OwnerDraw_ButtonVariant: text/vector, no SHP | decompile_function |
| 0x0060B000 | right-edge SHP-anchored button reposition; 0x93/0x9c insets; SHP hdr +2/+4 | decompile_function |
| 0x0069BBE0 | dialog-record flag `*(u8*)(rec+0x30D8)` (skin/stretch gate) | decompile_function |
| 0x0060B1D0/0060B350/0060B7A0/0060B950 | reposition helper family | get_function_by_address / decompile_function |
| 0x0060C0C0 | ResizeShellChildControl dispatcher (0xBBB -> 0060B7A0) | decompile_function |
| 0x0060C4A0 | stretch+resize entry (EnumChildWindows -> 0060C0C0) | decompile_function |
| 0x0060C540 | stretch gate; 0xBBB in recognized set -> TRUE | decompile_function |
| 0x00622820 | alternate WM_INITDIALOG body (also calls 0060C0C0) | decompile_function |
| 0x0060AAB0 | per-control attribute setter (rec+0xDC/+0xC8); NOT the dispatcher | decompile_function (fn created for nav) |
| 0x0060CAF0 / 0x0060C930 | init flag setters (rec+0xD9/+0xDA); no image | decompile_function |
| 0x008450F4 / 0x00845178 | "SIDEBTTN.SHP" / "SDBTNANM.SHP" strings in shell filename table | search_strings / read_memory |

## Notes / open items (UNCHECKED, not blocking)
- Exact skin-mode (rec+0xB0 = 1/2/3) for each of Back/Keyboard/Sound is data-driven at init and not
  individually transcribed; all three modes are SHP-frame paths (none paint via the dialog proc), so
  the routing conclusion holds regardless.
- A Ghidra function boundary was created at 0x0060AAB0 this session (DB-only navigation aid; no binary
  bytes changed, no labels renamed).
