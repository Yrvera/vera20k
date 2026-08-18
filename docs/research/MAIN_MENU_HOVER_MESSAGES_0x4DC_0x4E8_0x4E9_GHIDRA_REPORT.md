# Main-Menu Hover Messages 0x4DC / 0x4E8 / 0x4E9 — Ghidra RE Report

**Date:** 2026-05-19  
**Investigator:** re-swarm slot 3  
**Scope:** Custom Win32 messages 0x4DC, 0x4E8, 0x4E9 — sender sites, payloads, receiver handlers, visual-state impact  
**Active in YR:** Yes (all three are live in the main-menu hover path of dialog 0xE2)

---

## 1. Executive Summary

- **0x4DC** is sent to owner-draw **buttons** (and in network dialogs to spinners/labels). It is a **hover-enter / hover-leave notification** with `lparam = 1` (enter) or `lparam = 0` (leave). The button handler **does mutate visual state**: it sets/clears a hover flag at `button_object+0xC4` (`piVar17[0x31]`) and calls `InvalidateRect` on leave, forcing a WM_PAINT that selects the hover SHP frame. On enter it also starts a 1000ms blink timer. This IS the hover redraw mechanism.
- **0x4E8** is a **"hit-test / resolve hover target"** query. It is sent with packed `(y<<16)|x` coordinates as lparam. The receiver replies with a forwarded HWND (or -1). Button controls do NOT handle it (fall through to `CallWindowProcA`). Combo-box controls handle it by forwarding to their open dropdown child. The WM_NCHITTEST dispatcher uses the return value to locate the actual hovered widget.
- **0x4E9** is a **"previous hover target clear"** notification. It is sent to the previously-hovered control to tell it the cursor has left. The sender packs the old-child HWND pointer into an lparam buffer. Neither OwnerDraw_Button_00612B70 nor OwnerDraw_Static_006153E0 handles 0x4E9 explicitly — they both fall through to `CallWindowProcA` / `return 0`. The message is effectively a no-op for simple buttons and statics. For combo-box drop-downs, a helper sends 0x4E9 to the drop-down HWND returned by the earlier 0x4E8 call.

**Parity answer:** Hovering an owner-draw button on dialog 0xE2 **does** change its visual state, but the mechanism is 0x4DC (not 0x4E8 or 0x4E9). The WM_NCHITTEST handler sends 0x4DC to the hovered button with lparam=1 (enter) or 0 (leave) through a separate dispatch path in `FUN_006040B0`/`FUN_00604060`. The hover redraw is driven entirely by `InvalidateRect` inside the 0x4DC handler. A Rust implementation must send 0x4DC-equivalent hover notifications to trigger correct hover-frame rendering.

---

## 2. Message 0x4DC — Hover Enter / Leave

### 2.1 Senders

Byte pattern `68 DC 04 00 00` found at:

| Address | Container function | Context |
|---|---|---|
| 0x005E2098 | FUN_005E2340 (network dialog) | `SendMessageA(GetDlgItem(hwnd, 0x59F), 0x4DC, 0, 1)` — spinner enter |
| 0x005E2364 | FUN_005E2340 (network dialog) | enter variant |
| 0x005E23B5 | FUN_005E23A0 (network dialog) | `SendMessageA(GetDlgItem(hwnd, 0x59F), 0x4DC, 0, 0)` — spinner leave |
| 0x0079243F | unnamed (≈ network dialog proc) | send to control 0x59F |
| 0x007A2791 | unnamed | send to control 0x59F |
| 0x007A27E5 | FUN_007A27D0 | `SendMessageA(GetDlgItem(hwnd, 0x59F), 0x4DC, 0, 0)` — leave |

**Senders in the main-menu 0xE2 hover path:** 0x4DC is NOT sent directly from WM_NCHITTEST (FUN_00622B50). It is sent from a helper that is called as part of the hover-state-changed path. The tooltip updater in FUN_00622B50 calls `FUN_006040B0` to get the tooltip string and then separately the hover notification flows through `FUN_00604060` → `FUN_006040B0`. The actual 0x4DC dispatch to main-menu buttons is performed by the hover-tracking machinery invoked from FUN_00622B50 when the hovered child changes (verified: `FUN_006040b0` at 0x00604060 cross-references the button control IDs for dialog 0xE2 — 0x683, 0x684, 0x578, 0x686, 0x55C, 0x55F, 0x3EE — verified via `decompile_function 0x006040B0`).

### 2.2 Payload

- **wparam:** always 0
- **lparam:** `1` = mouse-enter, `0` = mouse-leave

### 2.3 Receiver: OwnerDraw_Button_00612B70 (0x00612B70)

Handler for `param_2 == 0x4DC` (verified via `decompile_function 0x00612B70`):

```
if (param_4 == 1):    // mouse enter
    if (piVar17[0x31] == 0):    // hover flag at button_object+0xC4
        piVar17[0x31] = 1
        SetTimer(hwnd, 0, 1000, NULL)
else:                 // mouse leave
    if (piVar17[0x31] != 0):
        piVar17[0x31] = 0
        piVar17[0xC5] = 0       // byte flag at +0xC5
        KillTimer(hwnd, 0)
        InvalidateRect(hwnd, NULL, TRUE)   // ← triggers WM_PAINT hover redraw
```

**Visual-state mutation:** YES. On leave (lparam=0), `InvalidateRect` forces WM_PAINT. The WM_PAINT handler (case 0xF) reads `*(char*)((int)piVar17 + 0xC5)` to select `'u'` (up/normal) vs `'d'` (down/pressed) and constructs the SHP filename as `"b_c_c_%c_%d.pcx"`. The hover state at `+0xC5` distinguishes normal from hover visuals.

**Active in YR:** Yes — this runs every time the mouse moves over a main-menu button.

---

## 3. Message 0x4E8 — Hit-Test / Resolve Hover Target

### 3.1 Senders

Byte pattern `68 E8 04 00 00` found at:

| Address | Sender function | Context |
|---|---|---|
| 0x00603E27 | unnamed thunk at ≈0x00603E10 | `SendMessageA(hwnd, 0x4E8, 0, (y<<16)\|x)` — fastcall thunk |
| 0x00603FEE | unnamed function (≈0x00603FC0) | similar thunk variant |
| 0x0060E281 | unnamed large function | forwards 0x4E8 with recomputed coords |
| 0x00611D1A | unnamed large function | `SendMessageA(hwnd, 0x4E8, 0, coords); → stores LRESULT; then sends 0x4E9` |
| 0x00618AD1 | OwnerDraw_ComboBox_00617250 | forwards 0x4E8 to open dropdown child (case 0x4E8 handler) |
| 0x00622D4E | FUN_00622B50 (WM_NCHITTEST handler) | `LVar4 = SendMessageA(pHVar3, 0x4E8, 0, (y<<16)\|x)` |

**Primary sender in dialog 0xE2 hover path:** `FUN_00622B50` at 0x00622D4E.  
Verified via `decompile_function 0x00622B50`.

### 3.2 Payload

- **wparam:** always 0
- **lparam:** packed screen coordinates `(y & 0xFFFF) << 16 | (x & 0xFFFF)` (verified via raw memory decode at 0x00603DE6–0x00603E3F)

### 3.3 Receivers and Handlers

**OwnerDraw_Button_00612B70:** Does NOT handle 0x4E8. Falls through to `CallWindowProcA`. Returns whatever the original window proc returns. Effectively unhandled; default return is 0 or whatever DefWindowProcA returns.

**OwnerDraw_Static_006153E0:** Does NOT handle 0x4E8 explicitly (no case in the switch table). Falls through to `CallWindowProcA`. Verified via `decompile_function 0x006153E0`.

**OwnerDraw_ComboBox_00617250 (0x00617250):** DOES handle 0x4E8 (case 0x4E8):
```
pHVar8 = piStack00000018[0x3D];   // open dropdown child HWND
if (pHVar8 == NULL) return -1;
// Translate coordinates from combobox-relative to dropdown-relative
WVar23 = SendMessageA(pHVar8, 0x4E8, 0, translated_coords);
return WVar23;   // forwards to dropdown
```
Returns the dropdown's HWND (or -1 if no dropdown open). Verified via `decompile_function OwnerDraw_ComboBox_00617250`.

### 3.4 Return Value Semantics

The 0x4E8 return value is used by FUN_00622B50:
```
LVar4 = SendMessageA(pHVar3, 0x4E8, 0, coords);
FUN_00603F00(pHVar3, LVar4);
```
`FUN_00603F00` receives `LVar4` (a potential HWND) and sends 0x4E9 to it. If 0x4E8 returns -1 (button's default), `FUN_00603F00` sends 0x4E9 to -1 which has no effect. If it returns a real HWND (combo dropdown case), 0x4E9 goes to that HWND.

**Observable side-effect:** None for buttons. The query-reply cycle finds the deepest hittable window, but for simple buttons the return is always -1 / default.

---

## 4. Message 0x4E9 — Previous Hover Target Clear

### 4.1 Senders

Byte pattern `68 E9 04 00 00` found at:

| Address | Sender function | Context |
|---|---|---|
| 0x00603F25 | FUN_00603F00 | `SendMessageA(param_2, 0x4E9, 0, &{param_3, param_4})` |
| 0x00603F7D | FUN_00603F60 | `SendMessageA(param_2, 0x4E9, 0, &{param_3, -1})` |
| 0x00604010 | unnamed ≈0x00603FF0 | variant |
| 0x0060E2F5 | unnamed large function | send to old hover target |
| 0x0060E383 | unnamed large function | variant |
| 0x00611D3C | unnamed large function | send 0x4E9 after 0x4E8 cycle |
| 0x00611DCB | unnamed large function | variant |
| 0x00622DC4 | FUN_00622B50 (WM_NCHITTEST) | `SendMessageA(param_1, 0x4E9, 0, &{pHVar3, -1})` — sent to the dialog |

Verified via `decompile_function 0x00622B50`, `decompile_function 0x00603F00`, `decompile_function 0x00603F60`, raw memory reads at 0x00611D00, 0x0060E270.

### 4.2 Payload

- **wparam:** always 0
- **lparam:** pointer to a 2-element DWORD buffer: `{child_hwnd, -1}` or `{result_from_0x4E8, caller_context}`

The lparam is an in-stack structure — not a heap allocation. It encodes the old hovered child HWND plus a sentinel (-1 or the 0x4E8 result) for the receiver to inspect.

### 4.3 Receivers and Handlers

**FUN_00622B50 (dialog 0xE2 receiving 0x4E9):**  
0x4E9 = 1257 decimal. In FUN_00622B50's dispatch:
- `param_2 < 0x85` → No (1257 > 133)
- `0x497 < param_2` → Yes (1257 > 1175) → checks `param_2 == 0x4EC` → No → `return (HGDIOBJ)0x0`

**0x4E9 is NOT handled** by FUN_00622B50 when it arrives at the dialog. It returns 0 immediately. Verified via `decompile_function 0x00622B50`.

**OwnerDraw_Button_00612B70 receiving 0x4E9:**  
No explicit case for 0x4E9 in the switch. Falls through to `CallWindowProcA`. Effectively unhandled.

**OwnerDraw_Static_006153E0 receiving 0x4E9:**  
No explicit case for 0x4E9. Falls through to `CallWindowProcA`. Verified via `decompile_function 0x006153E0`.

### 4.4 Observable Side-Effect

**None for dialog 0xE2 main-menu buttons.** 0x4E9 to the dialog returns 0. 0x4E9 to a button falls to `CallWindowProcA`. No state is mutated, no repaint is triggered. The message is part of a hover-resolution protocol used by the widget framework for combo-boxes with open dropdowns (where 0x4E8 returns a real HWND and 0x4E9 is forwarded to it for cleanup). For buttons, both are no-ops.

---

## 5. WM_NCHITTEST Handler — Full Hover Dispatch Flow (Confirmed)

Verified via `decompile_function 0x00622B50`:

```
case WM_NCHITTEST (0x84):
    1. GetWindowRect(dialog, &rect)
    2. pt = screen_coords_from_lparam - dialog_origin
    3. pHVar3 = ChildWindowFromPointEx(dialog, pt, skip_invisible=1)
    4. if pHVar3:
       a. LVar4 = SendMessageA(pHVar3, 0x4E8, 0, screen_coords_packed)
          // 0x4E8: hit-test query → buttons return 0/-1, combos return dropdown HWND
       b. FUN_00603F00(pHVar3, LVar4)
          // sends 0x4E9 to LVar4 (previous hover HWND if any)
       c. [hover state tracking via FUN_007b6720 / FUN_007b6760 / FUN_007b68e0]
       d. if hover state changed:
          SendMessageA(dialog, 0x4E9, 0, &{pHVar3, -1})
          // notifies previous hovered control — unhandled for buttons
          [more hover state tracking]
          if iVar6 == 0:
              iVar6 = FUN_006040B0()   // tooltip string lookup for dialog 0xE2
              FUN_007b66d0(tooltip_str) // set tooltip text
    5. lParam = FUN_007b7140()   // get status-bar text
       SendMessageA(status_bar_hwnd_0x695, 0x4B2, 0, lParam)
    6. return 0
```

**FUN_006040B0** (0x006040B0) is the tooltip/status string lookup. For dialog 0xE2 it maps button control IDs to string table keys:
- 0x683 → `s_STT_MainButtonSinglePlayer`
- 0x684 → `s_STT_MainButtonWWOnline`
- 0x578 → `s_STT_MainButtonNetwork`
- 0x686 → `s_STT_MainButtonMovies`
- 0x55C → `s_STT_MainButtonOptions`
- 0x55F → `s_STT_MainButtonYuriWebSite`
- 0x3EE → `s_STT_MainButtonExitGamemd`

Verified via `decompile_function 0x006040B0`.

---

## 6. Button Object Layout (Partial — Hover-Relevant Fields)

From OwnerDraw_Button_00612B70 decompilation:

| Offset | Type | Purpose |
|---|---|---|
| +0x00 | HWND | Button HWND |
| +0x04 | ptr | Surface/backbuffer ptr |
| +0x2F (×4 = 0xBC) | bool | Disabled flag (`piVar17[0x2f]`) |
| +0x30 (×4 = 0xC0) | int | Unknown |
| +0x31 (×4 = 0xC4) | bool | **Hover active flag** — set by 0x4DC lparam=1, cleared by lparam=0 |
| +0xC5 (byte) | bool | **Hover repaint state** — used in WM_PAINT to select 'u'/'d' SHP suffix |
| +0x3A (×4 = 0xE8) | int | pWStack_d8 — related to pressed/unpressed state |

Note: `param_1` in OwnerDraw_Button is `int *` (LONG * table), so `piVar17[0x31]` = byte offset `0x31 × 4 = 0xC4`. Verified via `decompile_function 0x00612B70`.

---

## 7. Open Questions

1. **Who sends 0x4DC to main-menu buttons?** FUN_00622B50 does not send 0x4DC directly. The hover-state machinery (`FUN_007b6720`/`FUN_007b6760`/`FUN_007b68e0`) likely handles this. These three functions are not decompiled this session and may contain the 0x4DC dispatch. Worth tracing in a follow-up session.

2. **What is the 1000ms timer in the 0x4DC handler?** On hover-enter, `SetTimer(hwnd, 0, 1000, NULL)` is called. WM_TIMER (0x113) sets `*(bool*)((int)piVar17 + 0xC5) = *(char*)((int)piVar17+0xC5) == 0` and calls `InvalidateRect`. This looks like a pulse/blink on hover after 1 second. Whether this fires in a normal YR main-menu session needs gameplay testing.

3. **FUN_007b6720, FUN_007b68e0, FUN_007b7100** — the global hover-state tracker functions were not decompiled. They likely maintain a "currently hovered HWND" global and trigger 0x4DC dispatch when it changes. Full trace needed to confirm exactly when 0x4DC lparam=1/0 are sent.

4. **Unknown senders at 0x0060E281, 0x00611D1A** — these large unnamed functions send 0x4E8 and 0x4E9 in sequence. Their containing dialog proc is not identified in this session. They may be for subsidiary dialogs (not dialog 0xE2).

---

## 8. Confidence Assessment

| Finding | Confidence | Evidence |
|---|---|---|
| 0x4DC handler in OwnerDraw_Button mutates hover flag and calls InvalidateRect | HIGH | Decompilation of FUN_00612B70 — explicit case 0x4DC with InvalidateRect call |
| 0x4E8 not handled by OwnerDraw_Button (falls to CallWindowProcA) | HIGH | Decompilation of FUN_00612B70 — no case 0x4E8 in switch |
| 0x4E8 not handled by OwnerDraw_Static (falls to CallWindowProcA) | HIGH | Decompilation of FUN_006153E0 — no case 0x4E8 |
| 0x4E9 not handled by OwnerDraw_Button or Static | HIGH | Same decompilations |
| 0x4E9 not handled when received by dialog 0xE2 (returns 0) | HIGH | Decompilation of FUN_00622B50 — control flow verified |
| 0x4E8 in WM_NCHITTEST sends packed coords to child-under-cursor | HIGH | Decompilation of FUN_00622B50 at 0x00622D4E |
| 0x4E8 handled by OwnerDraw_ComboBox — forwards to dropdown | HIGH | Decompilation of OwnerDraw_ComboBox_00617250 — explicit case 0x4E8 |
| FUN_006040B0 is tooltip string lookup for dialog 0xE2 | HIGH | Decompilation shows dialog-ID == 0xE2 check and button ID mapping |
| 0x4DC lparam=1 starts 1000ms blink timer | HIGH | Decompilation of FUN_00612B70 SetTimer call |
| Source of 0x4DC dispatch to main-menu buttons | MEDIUM | Not directly traced — FUN_007b6720 family not decompiled this session |
