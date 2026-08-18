# Hover Dispatcher Investigation — FUN_007b66c0 / 6720 / 6760 / 68e0 + 0x4DC Origin

**Investigation date:** 2026-05-19
**Anchor doc:** `MAIN_MENU_HOVER_MESSAGES_0x4DC_0x4E8_0x4E9_GHIDRA_REPORT.md` Open Question 1
**Status:** COMPLETE — both halves of the question resolved
**Active in YR:** Conditional (see §1)
**Verification mode:** Ghidra MCP read-only

---

## 1. Executive Summary

Two related questions investigated together:

**Q-A (the family identity):** What are `FUN_007b66c0`, `FUN_007b6720`, `FUN_007b6760`, `FUN_007b68e0` — are they the hover-state tracker hypothesized by the anchor doc?

→ **No.** They are a tiny `std::wstring`-like wide-string utility family (init, duplicate, free, assign). They are called from `FUN_00622B50`'s WM_NCHITTEST handler to manage the **tooltip text** stack-buffer used to format the bottom-left status line, not to track hover state. The anchor doc's hypothesis is refuted in full.

**Q-B (the original parity question):** Who sends `0x4DC` to main-menu owner-draw buttons on dialog `0xE2`?

→ **Nobody.** `0x4DC` is **not a hover message at all** — it is an **input-validation error-state notification** used exclusively by two network-dialog source files (`netdlg2.cpp` and `wonline.cpp`) targeting control `0x59F` (a network-dialog edit/spinner). All six `push 0x4DC` sites are network-dialog validators. Dialog `0xE2` does not contain control `0x59F`, and no other path reaches it.

**Parity consequence (the actual answer the user needs):**

> Hovering a main-menu owner-draw button on dialog `0xE2` has **NO visual effect** in gamemd.exe. The Rust port does not need to send any hover-enter/leave notifications, does not need to track hover state on the button object, and does not need a hover redraw path. `SHELL_BUTTON_PAINT_DETAILS_GHIDRA_REPORT.md` Q4 RESOLVED ("no WM_MOUSEMOVE handler, no hover state mutation, no hover-art-swap") is **correct** for main-menu PCX buttons.

> The press state (mouse-down) DOES change the button's `'u'` → `'d'` first PCX-filename char, but that is the `piVar17[0x3a] & 1` (pressed bit) path, not the `0x4DC` path.

**Active in YR:**
- `0x4DC` mechanism itself: **Conditional** — active when a YR player opens a network/WOL dialog and submits invalid input on control `0x59F`. Dormant in offline play.
- Main-menu (`0xE2`) involvement: **No.**

This finding **contradicts and corrects** the prior swarm report:

> `MAIN_MENU_HOVER_MESSAGES_0x4DC_0x4E8_0x4E9_GHIDRA_REPORT.md`'s "parity answer" claim that "Hovering an owner-draw button on dialog 0xE2 DOES change its visual state, driven entirely by 0x4DC" is **WRONG**. See §6 for the cross-doc reconciliation list.

---

## 2. The wstring utility family (Q-A — refutation)

All four functions decompile to small wide-string ops. The anchor doc's "string-state push/pop/commit" characterization was closer to correct than its later "hover-state tracker" hypothesis.

### 2.1 `FUN_007b66c0` — init / clear pointer slot

```c
void __fastcall FUN_007b66c0(undefined4 *param_1)
{
    *param_1 = 0;        // zero the wstring slot
}
```
Used at WM_NCHITTEST entry to clear a local wstring buffer before population.

Evidence: `decompile_function 0x007b66c0`.

### 2.2 `FUN_007b6720` — wstrdup (allocate-and-copy)

```c
undefined4 * __thiscall FUN_007b6720(undefined4 *param_1, int *param_2)
{
    *param_1 = 0;
    if (*param_2 != 0) {
        iVar1 = FUN_007ca405(*param_2);          // wcslen
        pvVar2 = operator_new(iVar1 * 2 + 2);    // bytes = (len+1)*2 for UTF-16
        *param_1 = pvVar2;
        FUN_007ca489(pvVar2, *param_2);          // wcscpy
    }
    return param_1;
}
```

Allocation size `len*2 + 2` confirms UTF-16 (2 bytes/char, 1 wchar null terminator).

Evidence: `decompile_function 0x007b6720`.

### 2.3 `FUN_007b6760` — wstring free / clear

```c
void __fastcall FUN_007b6760(int *param_1)
{
    if (*param_1 != 0) {
        FUN_007c8b3d(*param_1);   // operator delete
    }
    *param_1 = 0;
}
```

Used at WM_NCHITTEST exit (Step 11 in tooltip-flow doc) to release the temporary wstring.

Evidence: `decompile_function 0x007b6760`.

### 2.4 `FUN_007b68e0` — wstring assign / replace

```c
int * __thiscall FUN_007b68e0(int *param_1, int *param_2)
{
    // If old == new (FUN_007ca5d3 == wcscmp), early-return.
    // Otherwise: free old, allocate new = (wcslen(new)+1)*2 bytes, copy.
    // If new == NULL, use DAT_00887734 (empty wstring literal) as source.
}
```

Replaces the wstring at `*param_1` with a copy of `*param_2`. The `DAT_00887734` fallback is a static empty L"" string.

Evidence: `decompile_function 0x007b68e0`.

### 2.5 Anchor doc reconciliation

`MAIN_MENU_DIALOG_0XE2_TOOLTIP_HOVER_FLOW_GHIDRA_REPORT.md` Open Question 1 said: *"The string-state push/pop/commit functions used by the WM_NCHITTEST handler. Their exact stack discipline is not traced."* — That description is essentially correct; they are wstring lifecycle ops for the tooltip text buffer, NOT for hover state.

`MAIN_MENU_HOVER_MESSAGES_0x4DC_0x4E8_0x4E9_GHIDRA_REPORT.md` Open Question 1 then re-cast the same functions as the "global hover-state tracker" that dispatches `0x4DC` — that was a wrong hypothesis built on the misleading variable name `hover-tracking machinery`. **Refuted.**

---

## 3. The actual `0x4DC` senders (Q-B — full enumeration)

### 3.1 Sender population

| Search pattern | Hits | Notes |
|---|---|---|
| `68 DC 04 00 00` (push imm32) | **6** | full enumeration |
| `B8 DC 04 00 00` (mov eax, imm32) | 0 | no register-load forms |
| `BA DC 04 00 00` (mov edx, imm32) | 0 | "" |
| `B9 DC 04 00 00` (mov ecx, imm32) | 0 | "" |

So six `push 0x4DC` sites total. Verified via `search_byte_patterns`.

### 3.2 Per-site identity

| Address | Container | Target HWND | lparam | Semantic |
|---|---|---|---|---|
| `0x005e2098` | inline in uncreated function | `GetDlgItem(_, 0x59F)` | `0` (leave/clear) | netdlg2.cpp clear-error |
| `0x005e2364` | `FUN_005e2340` (named) | `GetDlgItem(_, 0x59F)` | `1` (enter/set) | netdlg2.cpp set-error |
| `0x005e23b5` | `FUN_005e23a0` (named) | `GetDlgItem(_, 0x59F)` | `0` (leave/clear) | netdlg2.cpp clear-error |
| `0x0079243f` | inline in uncreated function | `GetDlgItem(EDI, 0x59F)` | `0` (clear) | wonline.cpp clear-error |
| `0x007a2791` | `FUN_007a2750` (named) | `GetDlgItem(_, 0x59F)` | `1` (set) | wonline.cpp set-error |
| `0x007a27e5` | `FUN_007a27d0` (named) | `GetDlgItem(_, 0x59F)` | `0` (clear) | wonline.cpp clear-error |

All six target the **same control ID `0x59F`**. Verified via `decompile_function` on the four named functions and `read_memory` at the two inline sites (`0x005e2090` and `0x00792430`) — both inline sites show the canonical `push 0; push 0; push 0x4DC; push HWND; call [SendMessageA]` lparam=0 clear sequence.

### 3.3 The smoking gun — source-file string xrefs

The two named "set-error" functions (`FUN_005e2340`, `FUN_007a2750`) each load a localized error message after sending `0x4DC`:

```c
// FUN_005e2340
SendMessageA(pHVar1, 0x4dc, 0, 1);
uVar3 = StringTable__LoadString(s_D__ra2mdpost_netdlg2_cpp_00831288, 0x14b4);
FUN_005e2700(0xffffffff, uVar3);

// FUN_007a2750
SendMessageA(pHVar1, 0x4dc, 0, 1);
uVar3 = StringTable__LoadString(s_D__ra2mdpost_wonline_cpp_0084a430, 0x7212);
FUN_005e2700(0xffffffff, uVar3);
```

The string-table keys are the **original Westwood source file paths**:
- `D:\ra2md\post\netdlg2.cpp` (line 0x14B4)
- `D:\ra2md\post\wonline.cpp` (line 0x7212)

These are network-dialog source files. Combined with the consistent target `0x59F` and the matched "set-then-load-error-message" pattern, the semantic is unambiguous: **`0x4DC` is the per-control error-blink set/clear notification**, not a hover message.

### 3.4 Dialog `0xE2` has no `0x59F` control

Per the RT_DIALOG parse in `MAIN_MENU_DIALOG_0XE2_FULL_VISIBLE_COMPOSITION_GHIDRA_REPORT.md`, the dialog `0xE2` control list is:
- Buttons: `0x683`, `0x684`, `0x578`, `0x686`, `0x55C`, `0x55F`, `0x3EE`
- Bink movie static: `0x71A`
- Other statics: `0x71B`, `0x71C`, `0x694`, `0x695`, `0x71D`

Control `0x59F` is **not present**. `0x55F` (Yuri website button) is the closest numerical neighbor but is a different ID.

No `SetWindowsHookEx` / `BroadcastMessage` / `EnumChildWindows` path was found that would route `0x4DC` to a `0xE2` child by any indirect mechanism — all six senders use direct `GetDlgItem(_, 0x59F)` resolution against a network-dialog HWND.

---

## 4. Receiver behavior — exact details (this is the tiny-details section)

`OwnerDraw_Button_00612B70 @ 0x00612B70` receives both `0x4DC` and `WM_TIMER (0x113)`. The slot-3 report described this at a high level but missed several parity-relevant details.

Per-control record layout (relative to `piVar17` = bucket+1, where `piVar17` is `int *`):

| Offset (bytes) | Type | Access notation | Purpose |
|---|---|---|---|
| +0xBC | byte | `piVar17[0x2f]` | Disabled flag |
| +0xC0 | int? | `piVar17[0x30]` | unknown |
| +0xC4 | byte | `piVar17[0x31]` | **Error-state active boolean** (`0x4DC` set/clear) |
| +0xC5 | byte | `((int)piVar17 + 0xc5)` direct cast | **Blink-frame state** (toggled by WM_TIMER) |
| +0xB0 | int | `piVar17[0x2c]` | **Button visual variant** (`iVar14`): 0 = PCX, 1/2/3 = SHP-based |
| +0xE8 | int | `piVar17[0x3a]` | **Pressed bit** (LSB) → `pWStack_d8 & 1` in paint code |

Cast note: `piVar17[0x31]` uses C-style indexing on `int *` so the byte offset is `0x31 * 4 = 0xC4`. `(int)piVar17 + 0xc5` is the explicit-cast direct byte offset (`+0xC5`). These are two **adjacent bytes**.

### 4.1 `0x4DC` handler (case body, verbatim from decompile)

```c
else if (param_2 == 0x4dc) {
    // (re-walk hash table for piVar17; same record lookup as entry)
    if (param_4 == 1) {
        if ((char)piVar17[0x31] == '\0') {
            *(undefined1 *)(piVar17 + 0x31) = 1;
            SetTimer(param_1, 0, 1000, (TIMERPROC)0x0);
        }
    }
    else if ((char)piVar17[0x31] != '\0') {
        *(undefined1 *)(piVar17 + 0x31) = 0;
        *(undefined1 *)((int)piVar17 + 0xc5) = 0;
        KillTimer(param_1, 0);
        InvalidateRect(param_1, (RECT *)0x0, 1);
    }
}
```

Tiny details (parity-relevant):

1. **Set side is gated on current state being clear.** `if (piVar17[0x31] == 0)` — calling `SendMessageA(hwnd, 0x4DC, 0, 1)` a second time while error is already active does **nothing**. No timer restart, no `InvalidateRect`. Idempotent.

2. **Clear side is gated on current state being set.** Same logic in reverse. Calling clear on a clear control does nothing — no `InvalidateRect`.

3. **`SetTimer` ID is 0.** The Bink decode timer on the static `0x71A` uses ID `0x65`. Different control HWNDs anyway, so no collision possible. ID 0 means `SetTimer` returns a system-assigned ID, but the call site uses `0` literally — and `KillTimer(hwnd, 0)` later. This is the standard "timer with ID 0 on this window" pattern.

4. **Timer period = 1000 ms.** Toggle every 1 second → 2-second full blink cycle.

5. **`SetTimer`'s timer-proc arg is NULL.** So WM_TIMER messages go to the window's message queue and are handled by the OwnerDraw_Button itself, not by a callback.

6. **Clear path resets BOTH bytes (`+0xC4` AND `+0xC5`).** This is the only place `+0xC5` is reset to 0 outside the WM_TIMER toggle.

7. **`InvalidateRect(..., bErase=1=TRUE)` on clear** — the background fills before paint. (The WM_TIMER call also uses `bErase=1`.) Both force a full re-erase, not a sticky-pixel composite.

8. **NO matching code path sets `+0xC5 = 1` directly on enter.** The error state begins with `+0xC5 = 0` (= "off frame"), and the first 1-second tick toggles it to 1. So **the visible blink starts as "OFF" for 1 second after error trigger**, then flips to "ON" — there is a 1-second delay before the first visible alternate frame.

9. **The receiver re-walks the hash table inside the case.** It does not rely on the entry-time lookup. This is defensive — in case `piVar17` was invalidated. Functionally equivalent for normal use.

10. **No bounds check on lparam.** `param_4 != 1` is treated as "clear". Sending `0x4DC` with lparam=`2` or any other non-1 value clears (not just 0). Senders only use 0 or 1, but this is the actual receiver semantic.

### 4.2 `WM_TIMER` (0x113) handler

```c
if (param_2 < 0x114) {
    if (param_2 != 0x113) {
        switch(param_2) { /* ... */ }
    }
    // param_2 == 0x113 (WM_TIMER):
    *(bool *)((int)piVar17 + 0xc5) = *(char *)((int)piVar17 + 0xc5) == '\0';
    InvalidateRect(param_1, (RECT *)0x0, 1);
}
```

Tiny details:

1. **The toggle uses `== '\0'` comparison.** `*ptr = (*ptr == 0)`. Treating any non-zero value as "set" — so once `+0xC5` is ever non-zero (e.g., 1), the next tick goes to 0; the cycle is binary regardless of starting value.

2. **No filter on `wParam` (timer ID).** The handler does not check whether the WM_TIMER is from timer id 0 (the blink timer) vs any other timer ID that might be set on this HWND. There is no other `SetTimer` call inside `OwnerDraw_Button_00612B70`, so this is safe in practice — but if a parent dialog ever installs another timer on a button HWND, the toggle would fire on every tick of every timer. Not currently a bug.

3. **`InvalidateRect` with `bErase = 1`** — same as the clear path.

### 4.3 `WM_PAINT` (case 0xF) — when `+0xC5` actually matters

This is the parity-critical detail the prior swarm report missed.

The button has three visual modes selected by `iVar14 = piVar17[0x2c]`:

- **`iVar14 == 0`**: PCX-based (`bue_li/mi/ri%d.pcx` / `bde_li/mi/ri%d.pcx` family) — **this is what main-menu uses**.
- **`iVar14 == 1`**: SHP-based (`g_SDBTNANM_SHP`) — used by Minimap_Button et al.
- **`iVar14 == 2`**: SHP-based (`DAT_00B0F9EC`) — used by other shell buttons.
- **`iVar14 == 3`**: SHP-based (`DAT_00B0FACC`) — used by other shell buttons.

In the **PCX path (`iVar14 == 0`)**, the relevant code:

```c
pWVar16 = (WNDPROC)((uint)pWStack_d8 & 1);   // pressed bit
uStack_f4._3_1_ = 'u';
if (pWVar16 != (WNDPROC)0x0) {
    uStack_f4._3_1_ = 'd';
}
// ...
FUN_007c8ef4(auStack_40, s_b_c_c_li_d_pcx_0083589c,
             uStack_f4._3_1_, 0x65, piVar18);
```

The first format-string `%c` is `'u'` or `'d'` based **only on `pWStack_d8 & 1`** (= `piVar17[0x3a] & 1`, the pressed bit). The byte `+0xC5` is **never read** in this branch. Confirmed by full decompile.

In the **SHP paths (`iVar14 == 1/2/3`)**, `+0xC5` IS read:

```c
if (iVar14 == 1) {
    // ...
    if (((uint)pWStack_d8 & 1) == 0) {
        if (*(char *)((int)piVar17 + 0xc5) != '\0') {
            local_f0 = (WNDPROC)0x3;   // alternate frame for SDBTNANM
        }
    }
}
else if (iVar14 == 2) {
    if (((uint)pWStack_d8 & 1) == 0) {
        cVar3 = *(char *)((int)piVar17 + 0xc5);
        local_f0 = (cVar3 != 0) ? (WNDPROC)2 : (WNDPROC)0;
    }
}
else if (iVar14 == 3) {
    if (((uint)pWStack_d8 & 1) == 0) {
        cVar3 = *(char *)((int)piVar17 + 0xc5);
        local_f0 = (cVar3 != 0) ? (WNDPROC)2 : (WNDPROC)0;
    }
}
```

So the blink alternates the SHP **frame index**, not the asset itself. `+0xC5 = 0` → base frame; `+0xC5 = 1` → +2 (or +3 for iVar14 == 1) frame offset.

**Parity implication:** the blink is a **frame swap on SHP-based shell buttons only**, and only when not pressed. For PCX-based main-menu buttons, the blink path is dead-output — the timer toggles `+0xC5` but no paint code reads it.

### 4.4 First-paint default state

`+0xC5` on a freshly-allocated record is `0` (the bucket data zone is implicitly zeroed). So the first paint always selects the "off" frame. Confirmed by absence of any `+0xC5 = 1` write outside the WM_TIMER toggle.

---

## 5. Edge cases checked (all benign for main-menu)

| Edge case | Behavior | Source |
|---|---|---|
| `0x4DC` sent before button has a record | Receiver re-walks hash table; if record not found, `piVar17` stays NULL → out-of-bounds write would happen. But in practice the dialog proc owns the HWND lifecycle; record is allocated before any user input. | decompile_function 0x00612B70 |
| `0x4DC` sent during paint | No special handling; `0x4DC` case has no `BeginPaint`/`EndPaint` guard. The `InvalidateRect` it issues just adds to the dirty region. | same |
| `0x4DC` with `lparam = 2/3/...` | Treated as clear (since `param_4 != 1` → clear branch). | case body |
| Double-set (`0x4DC` lparam=1 twice in a row) | Idempotent; no second `SetTimer`, no extra paint. | guarded by `piVar17[0x31] == 0` |
| Double-clear | Idempotent; no `KillTimer`, no `InvalidateRect`. | guarded by `piVar17[0x31] != 0` |
| Timer races with clear | `KillTimer` removes pending WM_TIMER from queue; explicit `+0xC5 = 0` write follows; `InvalidateRect` repaints. Safe. | clear branch |
| Disabled control | `piVar17[0x2f]` is checked in `WM_PAINT` (case 0xF) at entry: if disabled, calls `ValidateRect` and returns 0. So a disabled button does NOT paint a blink even if its timer ticks. | case 0xF top |
| Control destroyed mid-blink | Hash table walk would fail to find the record, `piVar17` stays at entry value, but `param_1 == HWND` matches → record found. After `DestroyWindow`, the bucket would be freed by the dialog teardown. The blink timer would also be killed by `DestroyWindow` (Win32 cleans timers automatically). | Win32 semantics + record lifecycle |

None of these edge cases affect main-menu since `0x4DC` is never delivered there. They are documented for the network-dialog parity case where `0x4DC` matters.

---

## 6. Cross-doc reconciliation — corrections proposed (not auto-applied)

This investigation contradicts several claims in the prior swarm report `MAIN_MENU_HOVER_MESSAGES_0x4DC_0x4E8_0x4E9_GHIDRA_REPORT.md`. Listing them for user-driven correction:

| Prior claim | Status | Correction |
|---|---|---|
| "0x4DC is sent to owner-draw buttons (and in network dialogs to spinners/labels)" | **wrong** | 0x4DC is sent ONLY to network-dialog control `0x59F`. Not to "buttons in general." |
| "Senders in the main-menu 0xE2 hover path: 0x4DC is NOT sent directly from WM_NCHITTEST. It is sent from a helper called as part of the hover-state-changed path." | **wrong** | 0x4DC is never sent on the main-menu `0xE2` path at all. It is purely a network-dialog validation message. |
| "Hovering an owner-draw button on dialog 0xE2 DOES change its visual state, but the mechanism is 0x4DC." | **wrong** | Hovering a main-menu button has NO visual effect. `0x4DC` is unrelated to hover. |
| "FUN_007b6720/68e0/6760 family — global hover-state tracker functions, not decompiled this session, likely contain the 0x4DC dispatch." | **wrong** | They are wstring utilities, not hover trackers. They contain no `SendMessage` of any kind. |
| "On hover-enter, SetTimer(hwnd, 0, 1000, NULL) is called." | **wrong semantic** | On error-set, SetTimer(hwnd, 0, 1000, NULL) is called. Trigger is input validation failure, not mouse hover. |
| "A Rust implementation must send 0x4DC-equivalent hover notifications to trigger correct hover-frame rendering." | **wrong recommendation** | Rust port should NOT send any hover notifications for main-menu buttons. Currently-correct: no hover state, no hover redraw. |
| "WM_TIMER (0x113) sets ... InvalidateRect. This looks like a pulse/blink on hover after 1 second." | **partially correct** | Correct that it's a blink, wrong about the trigger ("on hover" → on validation error). |
| "Whether this fires in a normal YR main-menu session needs gameplay testing." | **resolved** | Does not fire in a main-menu session. Static analysis is conclusive — no sender targets a `0xE2` control. |
| "FUN_007b6720, FUN_007b68e0, FUN_007b7100 — likely maintain a 'currently hovered HWND' global and trigger 0x4DC dispatch when it changes." | **wrong** | Wstring utilities. No global hover state. No 0x4DC dispatch. |

The prior report's accurate findings (preserve):

- ✅ `OwnerDraw_Button_00612B70` has an explicit `case 0x4DC` mutating `+0xC4` and (via WM_TIMER) `+0xC5`.
- ✅ `0x4E8` is a hit-test/resolve-target query (lparam = packed coords); unhandled by buttons/statics, handled by combo-box.
- ✅ `0x4E9` is a previous-target clear; unhandled by buttons/statics; no-op for main-menu.
- ✅ The WM_NCHITTEST handler (`FUN_00622B50`) sends `0x4E8` then `0x4E9`, but never `0x4DC`.
- ✅ `FUN_006040B0` is the tooltip-key lookup table for dialog `0xE2`.

---

## 7. Open Questions — Final State of Investigation Log

Seeded entries and resolution status (all entries closed):

- `[RESOLVED] Q1` — Who sends `0x4DC` to main-menu buttons? → Nobody. All 6 sender sites target network-dialog control `0x59F`. (evidence: `search_byte_patterns 68 DC 04 00 00` → 6 hits; decompile + memory-read on each)
- `[RESOLVED] Q2` — Are the FUN_007b6* family hover trackers? → No, wstring utilities (init/dup/free/assign). (evidence: `decompile_function` on `0x007b66c0/6720/6760/68e0`)
- `[RESOLVED] Q3` — Semantic of `0x4DC`? → Input-validation error-state set/clear notification. (evidence: source-file strings `netdlg2.cpp` / `wonline.cpp` loaded immediately after the send in named senders)
- `[RESOLVED] Q4` — Does the receiver mutate visual state on PCX main-menu buttons? → No. PCX path (iVar14 == 0) reads only pressed-bit, not `+0xC5`. (evidence: `decompile_function 0x00612B70` iVar14 == 0 branch)
- `[RESOLVED] Q5` — Timer period & cadence? → 1000 ms toggle → 2 s full blink cycle. First visible frame change happens 1 s after error-set (not immediately). (evidence: case body + WM_TIMER body)
- `[RESOLVED] Q6` — Timer-ID collision with Bink? → No. ID 0 on button HWND vs ID `0x65` on static `0x71A` HWND; different windows. (evidence: SetTimer call sites)
- `[RESOLVED] Q7` — `+0xC4` / `+0xC5` semantics? → `+0xC4` = error-state active boolean; `+0xC5` = blink frame state. Adjacent bytes. (evidence: receiver write sites)
- `[RESOLVED] Q8` — Lparam value for clear? → Any value other than `1` clears. Senders use `0`, but receiver is permissive. (evidence: `else if (... != 1)` branch)
- `[RESOLVED] Q9` — Dialogs sending `0x4DC`? → netdlg2.cpp and wonline.cpp validators only. (evidence: string-table file-path arguments)
- `[RESOLVED] Q10` — `0x59F` on dialog `0xE2`? → No, not in `0xE2`'s RT_DIALOG control list. (evidence: prior report RT_DIALOG parse)
- `[RESOLVED] Q11` — Indirect `0x4DC` delivery (PostMessage/Broadcast/EnumChildWindows)? → No, all six sites use direct `SendMessage(GetDlgItem(_, 0x59F), 0x4DC, ...)`. (evidence: `search_byte_patterns` register-load forms returned zero; sender disassembly)
- `[RESOLVED] Q12` — Visual frame mapping for `+0xC5` in `iVar14 = 1/2/3`? → frame index +2 (or +3 for iVar14 == 1) when `+0xC5 != 0` and not pressed. (evidence: `decompile_function 0x00612B70` iVar14 branches)
- `[RESOLVED] Q13` — First-paint default `+0xC5`? → `0` (record zero-init). (evidence: absence of any other writer)
- `[RESOLVED] Q14` — Idempotency of double-set / double-clear? → Both idempotent, guarded by current-state checks. (evidence: case body)
- `[RESOLVED] Q15` — Disabled button + blink? → Disabled controls early-exit from WM_PAINT (`piVar17[0x2f] != 0`) → no blink rendered even if timer ticks. (evidence: case 0xF top)
- `[RESOLVED] Q16` — Parity recommendation? → Main-menu does not need hover dispatch; SHELL_BUTTON_PAINT_DETAILS Q4-RESOLVED stands. (evidence: combined Q1-Q4)

- `[DEFERRED] Q17` — Exact enclosing dialog procs for the two uncreated-function sites (`0x005e2098`, `0x0079243f`). (category: `out-of-scope`; reason: confirmed via memory-read that they are identical `SendMessage(_, 0x4DC, 0, 0)` clear-error calls targeting `0x59F`, so enclosing-proc identity does not change the parity answer; next-step-if-pursued: `create_function` at the function boundary above each address, then `get_function_callers` to identify the parent dialog.)
- `[DEFERRED] Q18` — Which network/WOL dialogs actually instantiate control `0x59F`. (category: `out-of-scope`; reason: investigation is scoped to main-menu hover; the network dialog topology is not parity-relevant here; next-step-if-pursued: search RT_DIALOG resources for `0x59F` in resource trees.)
- `[DEFERRED] Q19` — Whether any subdialog of the main menu (Options, Skirmish lobby, Network setup) DOES instantiate `0x59F` and therefore enable the blink. (category: `requires-different-system-context`; reason: those are separate dialogs not on the `0xE2` proc; their RT_DIALOG control lists need their own audits; next-step-if-pursued: a `--area network-dialogs` or `--area options-dialog` scan.)

All `[OPEN]` items resolved or deferred with category. Zero-add pass: a final full re-read of the OwnerDraw_Button decompile and the 6 sender bodies produced no new questions. Investigation is exhaustive within scope.

---

## 8. Sources

**Ghidra MCP calls (read-only) this session:**

- `decompile_function 0x007b66c0` — wstring init
- `decompile_function 0x007b6720` — wstrdup
- `decompile_function 0x007b6760` — wstring free
- `decompile_function 0x007b68e0` — wstring assign
- `decompile_function 0x00612B70` — `OwnerDraw_Button` receiver (full body including WM_TIMER + 0x4DC + WM_PAINT)
- `decompile_function 0x005e2340` — netdlg2.cpp set-error sender
- `decompile_function 0x005e23a0` — netdlg2.cpp clear-error sender
- `decompile_function 0x007a2750` — wonline.cpp set-error sender
- `decompile_function 0x007a27d0` — wonline.cpp clear-error sender
- `decompile_function 0x005dbec0` — validator helper for FUN_005e2340
- `search_byte_patterns 68 DC 04 00 00` — full enumeration of `push 0x4DC` (6 hits)
- `search_byte_patterns B8/BA/B9 DC 04 00 00` — register-load forms (all zero)
- `search_byte_patterns DC 04 00 00` — broader dword scan (~48 hits, mostly struct offsets and dword constants; confirmed no missed `push` form)
- `get_function_by_address` on each sender container
- `get_function_callers 0x005e2340`, `0x007a2750` — caller identification
- `read_memory 0x005e2090` (32 bytes) — confirm inline `0x005e2098` site is `SendMessageA(ESI, 0x4DC, 0, 0)` clear
- `read_memory 0x00792430` (32 bytes) — confirm inline `0x0079243f` site is `SendMessageA(EDI, 0x4DC, 0, 0)` clear via `GetDlgItem(EDI, 0x59F)`

**Prior reports referenced (verified or corrected):**

- `MAIN_MENU_HOVER_MESSAGES_0x4DC_0x4E8_0x4E9_GHIDRA_REPORT.md` — corrected (see §6)
- `MAIN_MENU_DIALOG_0XE2_TOOLTIP_HOVER_FLOW_GHIDRA_REPORT.md` — wstring family characterization corroborated
- `SHELL_BUTTON_PAINT_DETAILS_GHIDRA_REPORT.md` — Q4 ("no hover state mutation") confirmed correct for main-menu
- `MAIN_MENU_DIALOG_0XE2_FULL_VISIBLE_COMPOSITION_GHIDRA_REPORT.md` — RT_DIALOG control list cross-checked (no `0x59F` present)
- `SHELL_BUTTON_GREYSCALE_COLORIZATION_GHIDRA_REPORT.md` — iVar14 branches cross-referenced

**String evidence:**

- `s_D__ra2mdpost_netdlg2_cpp_00831288` — `"D:\ra2md\post\netdlg2.cpp"` (network dialog source path)
- `s_D__ra2mdpost_wonline_cpp_0084a430` — `"D:\ra2md\post\wonline.cpp"` (WOL dialog source path)
- `s_b_c_c_li_d_pcx_0083589c` / `s_b_c_c_mi_d_pcx_0083588c` / `s_b_c_c_ri_d_pcx_0083587c` — main-menu PCX filename format strings

**Confidence:** HIGH on all findings. Every load-bearing address was directly decompiled or memory-read this session.
