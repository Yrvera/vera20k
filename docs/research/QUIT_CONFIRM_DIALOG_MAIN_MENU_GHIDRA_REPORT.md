# Main-Menu Quit Confirm Dialog - Ghidra Report

Date: 2026-05-18

Scope: when the player clicks the Quit/Exit button on the standard YR main menu
(dialog `0xE2`, button control `0x3EE`), a Yes/No-style confirmation dialog
appears. This report identifies the dialog template, the CSF string keys, the
WMCommand handler that pops it, the dialog procedure that handles the button
clicks, and the shutdown path the affirm branch invokes.

Anchored on: `MAIN_MENU_DIALOG_0XE2_FULL_VISIBLE_COMPOSITION_GHIDRA_REPORT.md`.

No Rust code, INI, or Ghidra annotations were modified.

## Executive Summary

The main-menu Quit button does NOT pop a dedicated Win32 `MessageBoxA`. It
routes through the engine's own generic CSF-driven message-box helper
`FUN_005D3490`, which uses the RT_DIALOG resource template `0x120`
(byte-verified 4-control template; see
`RT_DIALOG_0X120_RESOURCE_LAYOUT_GHIDRA_REPORT.md`). The caller selects this
template because it passes the third CSF string but leaves the fourth NULL.

The displayed prompt is `GUI:ExitAreYouSure`. The two visible buttons are
`TXT_OK` (confirm-quit) and `GUI:Cancel` (stay). Confirm-quit causes
`Main_Game @ 0x0052D9A0` to write Options to ra2md.ini, fade out, and return
0 to the outer game loop, which drops out of WinMain. No explicit
`PostQuitMessage` or `ExitProcess` is invoked in this path; shutdown is a
clean return-cascade from `Main_Game`.

Verified facts:

1. `0xE2` Quit button control `0x3EE` → return code `6` from `FUN_00531CC0`.
2. Case-`6` in `Main_Game @ 0x0052D9A0` (starts at `0x0052DDBA`) constructs
   the confirm via `FUN_005D3490`.
3. `FUN_005D3490` selects dialog template `0x120` (4-control template per
   byte-verified parse; the caller leaves slot `0x5AF` unpopulated) at
   `0x005D3535` because `param_3` non-NULL and `param_4` NULL.
4. Dialog proc `0x005D36A0` writes `0`, `1`, or `2` to the result pointer
   stored via `SetWindowLongA(hwnd, 8, ...)`.
5. Affirm branch (`return 0`) flows to case-`7`, fade-out
   `FUN_004A3C30`, music stop `FUN_00720EA0(1)`, and `return 0`. The outer
   `Main_Game @ 0x0048CCC0` loop exits when this returns 0.

Overall confidence: High for the dialog-template selection, CSF keys,
control IDs, return-code semantics, and shutdown cascade. Active in YR:
Yes. Distinct from in-game Abort Mission (different dialog and caller).

## Entry Point: Quit Button Click

From the anchor doc: the main-menu dialog `0xE2` proc `0x00531F60` handles
`WM_COMMAND` for control `0x3EE` by writing `6` to the result pointer
retrieved via `GetWindowLong(hwnd, 8)`. The proc then exits the modal pump
when the result is set.

`FUN_00531CC0` (which calls `CreateDialogIndirectParamA` for dialog `0xE2`
with proc `0x00531F60`) returns that `6` back to its caller,
`Main_Game @ 0x0052D9A0`, where it lands in case-`6`.

## Case-6 Dispatch in `Main_Game @ 0x0052D9A0`

Disassembly window (decimal line numbers in CSF lookups are
`Init.CPP:2502/2503/2504`):

```
0x0052DDBA  PUSH 0x9C6                   ; Init.CPP line for error logging
0x0052DDBF  PUSH 0x825FB8                ; "D:\ra2mdpost\Init.CPP"
0x0052DDC4  XOR  EDX, EDX                ; out-ptr = NULL
0x0052DDC6  MOV  ECX, 0x826368           ; key = "GUI:ExitAreYouSure"
0x0052DDCB  CALL StringTable__LoadString
0x0052DDD2  MOV  ECX, 0x825FB0           ; key = "TXT_OK"
0x0052DDE3  CALL StringTable__LoadString
0x0052DDEA  MOV  ECX, 0x82635C           ; key = "GUI:Cancel"
0x0052DDFB  CALL StringTable__LoadString
0x0052DE17  PUSH 0                       ; extra slot
0x0052DE18  PUSH 0                       ; param_4 (cancel-button text) = NULL
0x0052DE19  PUSH EBP                     ; param_3 = "GUI:Cancel" text
0x0052DE1A  PUSH EDI                     ; param_2 = "TXT_OK" text
0x0052DE1B  PUSH ESI                     ; param_1 = "GUI:ExitAreYouSure" text
0x0052DE1C  CALL FUN_005D3490            ; modal CSF message box
0x0052DE21  TEST EAX, EAX
0x0052DE23  JNZ  0x0052DE39              ; non-zero -> back to main menu
0x0052DE25  MOV  ECX, 0xA8EB60           ; this -> g_OptionsClass
0x0052DE2A  MOV  ESI, 7                  ; next state = 7 (exit/shutdown)
0x0052DE2F  CALL OptionsClass__WriteToINI ; 0x005FAD10
0x0052DE34  JMP  0x0052E446              ; flow into state 7
```

CSF key strings, verified via `inspect_memory_content`:

| Pool address | Key string         | Slot in `FUN_005D3490` |
|--:|---|---|
| `0x00826368` | `GUI:ExitAreYouSure` | param_1 -> control `0x5B0` |
| `0x00825FB0` | `TXT_OK`             | param_2 -> control `0x5AE` |
| `0x0082635C` | `GUI:Cancel`         | param_3 -> control `2`     |

The numeric literals `0x9C6 / 0x9C7 / 0x9C8` are **C++ source line numbers**
in `D:\ra2mdpost\Init.CPP` (used by `StringTable__LoadString` for `__FILE__
:__LINE__` error reporting), NOT CSF numeric IDs.

## Modal Helper `FUN_005D3490`

Signature (Ghidra): `int FUN_005D3490(short* title, short* body_btn,
short* ok_btn, short* cancel_btn, void* unused)`. Each pointer is a
UTF-16 CSF string already resolved by `StringTable__LoadString`. A NULL
or empty-string slot disables the corresponding control.

Template selection at `0x005D351D-0x005D353A`:

```
ECX = 0xCE              ; default: text-only template (0 active body buttons)
if (cancel_btn populated) ECX = 0x121     ; "all 3 text slots used" template
else if (ok_btn populated) ECX = 0x120    ; "first 2 text slots used" template
PUSH 0                  ; extra param
MOV  EDX, 0x005D36A0    ; dialog proc
CALL FUN_00622650       ; CreateDialogIndirectParamA wrapper
```

For main-menu Quit: `cancel_btn = NULL`, `ok_btn = "GUI:Cancel"`, so the
selected RT_DIALOG resource id is `0x120`.

Inside the modal pump (`0x005D34A8-0x005D3653`):

- `local_8` (return code) initialized to `-1`.
- `SetWindowLongA(hWnd, 8, &local_8)` installs the result pointer.
- For each populated slot, sends `SendMessageA(GetDlgItem(hwnd, ID), 0x4B2,
  0, text_ptr)` to populate the control text.
- `FUN_00622800` shows the dialog.
- Pump loop spins on `FUN_00623120()` (modal message handler) until
  `local_8 >= 0`.
- `FUN_00622720` tears the dialog down. `return local_8`.

If no button slots populated at all, `local_8` is forced to `0` and the
helper returns `0` immediately. Not the case for the Quit confirm (one
button slot active).

Confidence: High from full decompilation and disassembly of `FUN_005D3490`.

## Dialog Proc `0x005D36A0`

Handles `WM_COMMAND (0x111)` via:

```
hi  = HIWORD(wParam)                   ; notification code
lo  = LOWORD(wParam) & 0xFFFF          ; control id
ptr = GetWindowLongA(hwnd, 8)          ; result-pointer slot

if (lo == 0x5AE && hi == 0) *ptr = 0   ; "TXT_OK"  body-slot button
if (lo == 1 || lo == 2)     *ptr = 1   ; "GUI:Cancel" OK-slot button
if (lo == 0x5AF && hi == 0) *ptr = 2   ; (0x5AF exists in 0x120 template
                                          but caller leaves text unpopulated;
                                          handler is reachable if user clicks)
```

Falls back through common shell proc `FUN_00622B50` for non-`WM_COMMAND`
messages, and through `FUN_00776D80` for `WM_MOUSEWHEEL (0x216)`.

Confidence: High from full disassembly of `0x005D36A0`.

## Branching in Case-6

| `FUN_005D3490` return | Source action          | Next state in Main_Game |
|---:|---|---:|
| `0` | Click "TXT_OK" body button   | `ESI = 7`  -> exit/shutdown |
| `1` | Click "GUI:Cancel" OK button | `ESI = 0x12` -> main menu  |
| `2` | Click control `0x5AF`        | (`0x5AF` exists in `0x120` template but the Quit-confirm caller leaves it visually empty) |

`SetWindowLong(hwnd, 8, ...)` is the same channel used by the main-menu
proc `0x00531F60` and other shell dialogs to plumb dialog results back to
the caller without `EndDialog`. See anchor doc section "Dialog proc
`0x00531F60`".

## Shutdown Path (Case-7 in Main_Game)

State `ESI = 7` re-enters the `Main_Game @ 0x0052D9A0` state switch and hits
case-`7`:

```
case 7:
  FUN_00720EA0(1);                      ; music: stop/fade
  iVar11 = DAT_00887340;
  if (DAT_00887338 != -1) iVar11 += GetRadarTimer() - DAT_00887338;
  cVar3 = FUN_00720FD0();               ; movie state? returns whether vox pumping
  goto joined_r0x0052E7A3;               ; tail flow

joined_r0x0052E7A3:
  if (!cVar3 || !VoxClass__PumpAndCheckActive())
    goto LAB_0052E7E6;                   ; jump to final-cleanup tail
  ...                                    ; wait up to 3s for vox to finish
  goto LAB_0052E7E6;

LAB_0052E7E6:
  FUN_00720EA0(0);                      ; music: stop
  if (!DAT_008175B0) FUN_004A3C30(0);   ; full screen fade to black
  return 0;                              ; <-- Main_Game returns 0
```

There is no `PostQuitMessage`, no `ExitProcess`, and no direct WinAPI exit
call on this path. Shutdown is a return cascade:

1. Inner `Main_Game @ 0x0052D9A0` returns `0` (false).
2. Outer `Main_Game @ 0x0048CCC0` `while (cVar1 != '\0')` loop exits.
3. Outer destroys display chain via `g_DisplayChain[0x14]` and returns.
4. `WinMain @ 0x006BB9A0` falls through to its normal cleanup epilogue.

`OptionsClass__WriteToINI` was already called in case-6 before the jump
into case-7, so the player's last skirmish/audio/video options persist.

Confidence: High from full decompilation of case-6, case-7, and the outer
loop.

## TS-vs-YR Filter

- `GUI:ExitAreYouSure` ASCII string at `0x00826368` is xref'd from exactly
  one site: `0x0052DDC6` (Main_Game case-6). It is NOT shared with any
  in-game Abort Mission code path.
- Dialog template IDs `0x120` / `0x121` / `0xCE` are immediates referenced
  only inside `FUN_005D3490` (verified by byte-pattern scans for
  `MOV ECX, imm32`). No other code path constructs these templates.
- `FUN_005D3490` is a generic engine helper called from 27 sites
  (`get_function_callers`), including save-load/error popups, network
  warnings, and CD-prompt cases. The Quit-confirm site is one of these,
  not a TS holdover.
- The TS-era in-game pause-menu Abort Mission dialog is OUT OF SCOPE for
  this slot - see Open Questions.
- No `SpecialFlags` or `g_GameMode` gating wraps the case-6 message box;
  it is reachable in every YR game mode (campaign, skirmish, network)
  whenever the user is on the main menu and clicks Quit.

YR-active: Yes.

## Resource Template `0x120` Note

**RESOLVED.** The RT_DIALOG byte parse was completed in a later swarm
slot — see `RT_DIALOG_0X120_RESOURCE_LAYOUT_GHIDRA_REPORT.md` for the full
verified layout (resource RVA, control rects in DLU, styles, font).

Summary from that report (corrects the by-behavior estimate previously
listed here):

- Plain `DLGTEMPLATE` (NOT the EX variant), style `0x40000040`
  (`WS_CHILD | DS_SETFONT`), rect `300×200 DLU`, font `MS Sans Serif 8pt`.
- **4 controls total** (not 3), all `WS_VISIBLE`:
  - Control `2` — Button `BS_OWNERDRAW`, rect `(207,175,83,15)`, baked
    title `L"GUI:Cancel"`, click result `1`.
  - Control `0x5B0` — Static `SS_LEFT`, rect `(40,40,220,50)`, baked title
    `L"GUI:Blank"`, receives runtime prompt via `SendMessageA(0x4B2)`.
  - Control `0x5AE` — Button `BS_OWNERDRAW`, rect `(207,135,83,15)`, baked
    title `L"GUI:OK"`, click result `0`.
  - Control `0x5AF` — Button `BS_OWNERDRAW`, rect `(207,155,83,15)`, baked
    title `L"GUI:Blank"`, click result `2`. **The Quit-confirm caller
    leaves this slot unpopulated (no `SendMessage` text update)**, but
    the control is physically present in the template and would render
    its baked `L"GUI:Blank"` title if not overwritten — behavior of the
    common-shell owner-draw path determines whether it appears as an
    empty button or is suppressed.

The owner-draw subclassing for these controls flows through the same
`FUN_00622B50` common-shell `WM_INITDIALOG` path documented in the anchor
doc. Confidence: High for IDs, styles, rects, and message-routing (all
byte-verified).

## Implementation Implications

For pixel-faithful main-menu Quit-confirm behavior:

1. The Quit button (control `0x3EE` on dialog `0xE2`) must produce a
   confirm prompt before any teardown.
2. Render CSF lookup keys: prompt = `GUI:ExitAreYouSure`, confirm-button
   label = `TXT_OK`, cancel-button label = `GUI:Cancel`. Use the live
   CSF data from `ra2md.csf` (via `langmd.mix`). Do NOT hardcode English
   text.
3. Default focus and ESC handling should match the engine's pump (ESC
   should resolve to cancel = stay on menu, not quit).
4. Confirm path must persist `ra2md.ini` via `OptionsClass__WriteToINI`
   semantics before shutdown.
5. The shutdown sequence is fade-to-black, vox pump (up to 3s), then
   graceful exit. No process-kill required.
6. Cancel path returns the user to dialog `0xE2` with no state change
   other than the modal child being destroyed.

## Open Questions

1. **RESOLVED.** The RT_DIALOG byte layout for resource `0x120` was
   re-parsed in a follow-up swarm — see
   `RT_DIALOG_0X120_RESOURCE_LAYOUT_GHIDRA_REPORT.md`. Body slot `0x5AE`
   is a `BS_OWNERDRAW` button (not Static); no control carries
   `BS_DEFPUSHBUTTON` (all three buttons are owner-draw).
2. The two-button template `0x121` is exercised by other callers (e.g.,
   save-game CD prompts). Whether its visual layout matches the Quit
   confirm's `0x120` rendering at a pixel level was not compared.
3. The in-game pause-menu Abort Mission dialog is a separate path with
   different control IDs and different shutdown semantics. It is OUT OF
   SCOPE for this report; flagged here as adjacent. Suggested follow-up:
   trace `g_GameState = 7` writers from the in-game sidebar to find that
   dialog's template and caller.
4. ESC-key-on-the-confirm modal behavior was not traced end-to-end; the
   pump in `FUN_00623120` likely synthesizes a default-button click, but
   which slot (confirm or cancel) gets the synthesized click was not
   verified.

## Sources

Functions decompiled/disassembled (Ghidra MCP, read-only, no rename or
save):

- `FUN_00531CC0` (anchor doc)
- `MainMenuDialog0xE2_Proc_00531F60` (anchor doc)
- `Main_Game @ 0x0052D9A0` (inner game loop, case-6 and case-7)
- `Main_Game @ 0x0048CCC0` (outer game loop / WinMain caller)
- `FUN_005D3490` (CSF message-box helper)
- `FUN_005D36A0` (message-box dialog proc)
- `FUN_00622650` (CreateDialogIndirectParamA wrapper)
- `FUN_00622B50` (common shell proc, anchor doc)
- `StringTable__LoadString @ 0x00734E60`
- `OptionsClass__WriteToINI @ 0x005FAD10`
- `FUN_004A3C30` (screen fade-out)

Binary/memory evidence:

- `0x00826368` = `"GUI:ExitAreYouSure"` (ASCII, NUL-terminated, 19 bytes)
- `0x00825FB0` = `"TXT_OK"` (ASCII)
- `0x0082635C` = `"GUI:Cancel"` (ASCII)
- `0x00825FB8` = `"D:\ra2mdpost\Init.CPP"` (ASCII; logging path)
- `MOV ECX, 0xCE / 0x120 / 0x121` immediates exist only inside
  `FUN_005D3490` (byte-pattern scan).

Prior reports referenced:

- `docs/research/MAIN_MENU_DIALOG_0XE2_FULL_VISIBLE_COMPOSITION_GHIDRA_REPORT.md`
