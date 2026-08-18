# Main Menu Dialog 0xE2 — Tooltip / Hover-Text Flow — Ghidra Report

Date: 2026-05-19

## Scope

Whether dialog `0xE2` (the initial YR main menu) shows any tooltip popup or
hover-text behavior when the mouse is over a shell button (Campaign, Skirmish,
Options, Quit, etc.), and if so, the mechanism. Specifically:

- Does `FUN_006040B0` (tooltip string registry) supply text for the main menu?
- What triggers updates to static `0x695` (the documented tooltip/status-line sink)?
- Is a Win32 TOOLTIPS_CLASS32 control, balloon tooltip, or TrackMouseEvent/
  WM_MOUSEHOVER mechanism used?
- Is the ToolTips INI option a gate for this behavior?

Out of scope: in-game cursor tooltips, sidebar cameo tooltips, skirmish-dialog tooltips.

No Rust files, INI files, in-repo docs/, or Ghidra annotations were modified.

## Executive Summary

Dialog `0xE2` **does show hover status-line text** when the mouse is over a button.
The mechanism is a **status-line update**, not a popup window or balloon tooltip:

- The **common shell dialog proc** `FUN_00622B50` handles `WM_NCHITTEST` (`0x84`)
  for all shell dialogs including `0xE2`.
- On every `WM_NCHITTEST` it hit-tests the child under the cursor, calls
  `FUN_006040B0` (the tooltip string registry) to look up the CSF key for the
  control, loads the localized string, and sends it via custom message `0x4B2`
  to control `0x695` (the owner-draw static at bottom-left, DLU `2,355,303,12`).
- Control `0x695` rerenders its backing surface on `0x4B2`/`0x4B4`.
- **No Win32 TOOLTIPS_CLASS32 control** is present — the string `"TOOLTIPS_CLASS"`
  does not appear in gamemd.exe's string table. No TTM_* messages, no
  TrackMouseEvent, no `WM_MOUSEHOVER`, no balloon window.
- The **ToolTips INI flag** (`[Options] ToolTips=`) is stored in `OptionsClass`
  at **byte offset 0x20** (decompile cites `*(undefined1 *)(param_1 + 8)`
  where `param_1` is `undefined4 *`, so the dword index of 8 maps to byte
  offset `8 × 4 = 0x20` per the CLAUDE.md `int *` pitfall rule; verified via
  `decompile_function 0x005FA87C`). It is read only by
  `OptionsClass__ReadFromINI` / `OptionsClass__WriteToINI`. The WM_NCHITTEST handler in `FUN_00622B50` does
  **not** check this flag; the shell status-line mechanism fires unconditionally.
  The ToolTips flag gates sidebar cameo tooltips and in-game tooltip windows,
  not the shell menu status line.
- Active in YR: **Yes** — `WM_NCHITTEST` fires on every mouse move over the
  dialog client area, making status-line updates continuous during hover.

## Verified Facts

### 1. FUN_006040B0 — Tooltip String Registry

**Address:** `0x006040B0`  
**Signature (Ghidra):** `char * __fastcall FUN_006040B0(int param_1, HWND param_2)`

The function takes a dialog identifier (looked up from a hash table via `param_1`)
and a child HWND. It calls `GetDlgCtrlID(param_2)` to get the control ID, then
dispatches on `(dialog_id, control_id)` and returns a pointer to an ASCII CSF key
string.

**Dialog `0xE2` branch (verified in decompilation, lines 31–54 of output):**

| Control ID | CSF key string |
|---|---|
| `0x683` | `s_STT_MainButtonSinglePlayer_00835784` |
| `0x684` | `s_STT_MainButtonWWOnline_0083576c` |
| `0x578` | `s_STT_MainButtonNetwork_00835754` |
| `0x686` | `s_STT_MainButtonMovies_0083573c` |
| `0x55C` | `s_STT_MainButtonOptions_00835724` |
| `0x3EE` | `s_STT_MainButtonExitGamemd_00835708` |
| `0x55F` | `s_STT_MainButtonYuriWebSite_00833de4` |
| all others | `(char *)0x0` (null — no tooltip) |

The `0x5EF` website button maps to `STT:MainButtonYuriWebSite` (confirmed from
the composition doc which notes this is not a ship button, only present when
`SpecialFlags` enables the web-link). When `FUN_006040B0` returns null, the
status line is cleared (set to a global empty wide string `&DAT_00887734`).

**Callers of `FUN_006040B0`** (from `get_function_callers`):
- `FUN_00604060 @ 0x00604060` — a thin wrapper that calls `006040B0` and then
  feeds the result to `StringTable__LoadString`. Called from 0x00791776,
  0x0078fe9e, 0x00791c0c (non-shell-menu paths).
- `FUN_00622B50 @ 0x00622B50` — the common shell proc, called from
  `MainMenuDialog0xE2_Proc_00531F60` on every message dispatch.

### 2. FUN_00622B50 WM_NCHITTEST Handler — the Trigger

**Address:** `0x00622B50`  
**Signature:** `HGDIOBJ __fastcall FUN_00622B50(HWND param_1, uint param_2, undefined4 param_3, HWND param_4)`

This is a shared ("common shell") message handler called first from every
shell dialog proc. When `param_2 == 0x84` (WM_NCHITTEST):

```
Step 1: GetDlgItem(param_1, 0x695)   → if null, skip entire handler
Step 2: Decode mouse pos from lParam (param_4 low/high words)
Step 3: GetWindowRect(param_1, ...) → subtract dialog origin
Step 4: FUN_007b66c0()               → save/push string stack state
Step 5: ChildWindowFromPointEx(param_1, pt, 1) → pHVar3 (child under cursor)
Step 6: SendMessageA(pHVar3, 0x4e8, 0, packed_pos) → query child hover state
Step 7: FUN_00603f00(pHVar3, LVar4)  → sends 0x4e9 to pHVar3 (clear hover state)
Step 8: Check string-state via FUN_007b7100(&DAT_00887734) → if empty (iVar6==0):
          a. Reset local_28 = -1
          b. SendMessageA(param_1, 0x4e9, 0, &{child=pHVar3}) → query dialog
          c. Check string-state again via FUN_007b7100()
          d. If still empty: call FUN_006040B0() → returns CSF key or null
          e. If non-null CSF key: StringTable__LoadString(..., 0x7a5) → wchar_t*
          f. FUN_007b66d0(puVar7) → store the string in local string-state buffer
Step 9: lParam = FUN_007b7140()      → read stored wide string ptr
Step 10: SendMessageA(local_18 /*=0x695*/, 0x4b2, 0, lParam) → update status line
Step 11: FUN_007b6760()              → pop string stack state
Step 12: return (HGDIOBJ)0x0
```

The `iVar6 == 0` check at step 8 falls through to `FUN_006040B0` only when the
child control itself has not supplied a custom hover string via `0x4e9`. For the
six PCX buttons on `0xE2`, no custom `0x4e9` handler is present — their
`OwnerDraw_Button_00612B70` does not handle `0x4e9` — so the fallback to
`FUN_006040B0` always fires for main menu buttons.

**No ToolTips check:** The WM_NCHITTEST handler has no conditional branch on
`OptionsClass.ToolTips` or any other global flag. It fires unconditionally.

### 3. Control 0x695 — Status-Line Sink

**Control ID:** `0x695`  
**Class:** Owner-draw Static, proc `OwnerDraw_Static_006153E0 @ 0x006153E0`  
**DLU rect:** `2, 355, 303, 12` → approximately `3, 577, 455, 20` px at 800×600  
**Dialog template title:** `GUI:Blank`  
**Initial display:** blank (CSF `GUI:Blank` resolves to the empty string or a
  single space — no visible content until hover)

Message `0x4B2` handler in `OwnerDraw_Static_006153E0` (case `0x4b2` / `0x4b4`):
```
iVar6 = piVar11[4]  // backing surface pointer
if (iVar6 != 0):
    FUN_00775690()
    GetClientRect(param_1, &rect)
    [build clip rect from stored geometry]
    (*DAT_00887310.vtbl[2])(clip, iVar6, dest, 0, 1)   // blit new surface
    InvalidateRect(apHStack_10[0], NULL, 0)
return 1;
```

The new text arrives via `lParam` to `0x4B2`. The store into the text buffer
happens through the `FUN_007b66d0/FUN_007b7140` string-state mechanism that
feeds the lParam, then causes a surface repaint. The result is the localized
CSF string drawn through `FUN_00621040` (BitFont/GAME.FNT path, yellow
`#FFFF00`, same font as button labels) at the next WM_PAINT.

### 4. No Win32 Tooltip Window — Verified Negative

String search of gamemd.exe for `"TOOLTIPS_CLASS"` → **zero matches**.  
String search for `"TTM_"` → **zero matches**.  
String search for `"TrackMouseEvent"` → **zero matches**.  
String search for `"WM_MOUSEHOVER"` → **zero matches**.

There is no `CreateWindowEx("tooltips_class32", ...)` anywhere in gamemd.exe.
The tooltip display mechanism is entirely internal: status-line update to `0x695`.

### 5. ToolTips INI Flag Scope

`[Options] ToolTips` (INI key `"ToolTips"` at `0x00833188`, `OptionsClass`
**byte offset 0x20** — decompile shows `*(undefined1 *)(param_1 + 8)` with
`param_1` typed `undefined4 *`, so the dword index 8 = byte 0x20; verified
via `decompile_function 0x005FA87C`) is read by
`OptionsClass__ReadFromINI @ 0x005FA87C` and written
by `OptionsClass__WriteToINI @ 0x005FAE33`. It controls per-entity in-game
tooltips (sidebar cameos, unit status overlays). The shell WM_NCHITTEST handler
at `FUN_00622B50` has **no reference** to this field; the shell status-line
behavior is always enabled regardless of the player's ToolTips preference.

## Mechanism Summary

```
Mouse moves over a button on dialog 0xE2
  → Win32 posts WM_NCHITTEST (0x84) to dialog
  → MainMenuDialog0xE2_Proc_00531F60 @ 0x00531F60 dispatches to FUN_00622B50
  → WM_NCHITTEST handler:
      - ChildWindowFromPointEx → child HWND under cursor (e.g., button 0x683)
      - FUN_006040B0(dialog_ref, child_hwnd) → "STT:MainButtonSinglePlayer"
      - StringTable__LoadString → wchar_t* localized tooltip text
      - SendMessage(hwnd_0x695, 0x4B2, 0, wchar_ptr) → update status-line text
  → OwnerDraw_Static_006153E0 on 0x4B2: reblit backing surface + InvalidateRect
  → WM_PAINT on 0x695: FUN_00621040 draws tooltip text in yellow GAME.FNT
```

**Trigger frequency:** Every WM_NCHITTEST message, which Win32 delivers on every
mouse move event over the dialog. High frequency (every frame of mouse motion).

## CSF Keys Used for Main Menu Buttons

| Button | CSF key (ASCII in .rdata) | Control ID |
|---|---|---|
| Single Player | `STT:MainButtonSinglePlayer` | `0x683` |
| WW Online | `STT:MainButtonWWOnline` | `0x684` |
| Network | `STT:MainButtonNetwork` | `0x578` |
| Movies & Credits | `STT:MainButtonMovies` | `0x686` |
| Options | `STT:MainButtonOptions` | `0x55C` |
| Exit Game | `STT:MainButtonExitGamemd` | `0x3EE` |
| Yuri Website | `STT:MainButtonYuriWebSite` | `0x55F` |

Controls `0x694` (heading), `0x71A` (movie), `0x71C`, `0x71D` (version) return
null from `FUN_006040B0` → `0x695` is cleared (empty wide string).

## Rust Implementation Notes

The Rust shell should:
1. On mouse move over the main menu, hit-test the child button.
2. Look up the CSF key from a table equivalent to `FUN_006040B0`'s 0xE2 branch.
3. Load the localized string via the CSF string table.
4. Update the status-line control `0x695` text with the result.
5. The trigger is mouse position change (equivalent to WM_NCHITTEST), not a
   timed hover or TrackMouseEvent delay — it fires on every mouse-move frame.
6. Do NOT implement a Win32-style tooltip popup window or balloon.
7. No ToolTips option gate — the status line always updates.

## Open Questions

- **`0x4e8` and `0x4e9` custom messages:** These are sent to child controls in
  the WM_NCHITTEST handler but their full semantics (beyond "hover state
  query/clear") are not traced. Neither `OwnerDraw_Button_00612B70` nor
  `OwnerDraw_Static_006153E0` appears to handle `0x4e8` explicitly — they likely
  fall through to `CallWindowProcA`. Confidence: medium (not verified in button
  proc for 0x4e8 explicitly, though the absence of a case block was observed).
- **`FUN_007b66c0 / FUN_007b6720 / FUN_007b6760 / FUN_007b68e0`:** The
  string-state push/pop/commit functions used by the WM_NCHITTEST handler. Their
  exact stack discipline is not traced. Behavioral output is confirmed (string
  is set and read back via `FUN_007b7140`).
- **`STT:` CSF keys:** The CSF string table values for `STT:MainButton*` keys
  are not verified in this session (they are in `ra2md.csf`, not in this
  binary). They contain the actual visible hover text (e.g., "Start a new
  single-player game").

## Confidence Levels

| Finding | Content | Identity | Binding |
|---|---|---|---|
| 0xE2 branch of FUN_006040B0 | HIGH — decompiled, saw all 7 control IDs and CSF key strings | HIGH — Ghidra label confirmed; xref from MainMenuDialog0xE2_Proc | HIGH — called by FUN_00622B50 which is called by the labeled proc |
| WM_NCHITTEST trigger path | HIGH — full decompiled flow visible | HIGH — message value 0x84 is a Win32 constant | HIGH — verified in FUN_00622B50 which is called from MainMenuDialog0xE2_Proc_00531F60 |
| Control 0x695 updated via 0x4B2 | HIGH — SendMessageA(0x695_hwnd, 0x4B2, 0, lParam) explicit | HIGH — from FUN_00622B50 WM_NCHITTEST block | HIGH |
| No TOOLTIPS_CLASS32 | HIGH — string search returned zero matches | HIGH — exhaustive string search | N/A |
| ToolTips flag not a gate | HIGH — OptionsClass__ReadFromINI decompiled, flag at byte offset 0x20 (dword index 8 of `undefined4 *param_1`); WM_NCHITTEST handler has no such check | HIGH | HIGH |
