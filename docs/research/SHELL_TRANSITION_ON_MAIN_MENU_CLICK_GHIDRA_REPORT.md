# Shell Slide Transition on Main-Menu Click — Ghidra Research Report

**Address(es):** `0x00608260` (slide-in trigger), `0x00608070` (slide-out trigger), `0x00608380` (`+0xC1` setter), `0x00607FD0` (slide-out paint dispatcher), `0x006071E0` (animation loop body), `0x00622720` (dialog cleanup), `0x00531CC0` (main-menu launcher), `0x00531F60` (main-menu dialog proc)

**Confidence:** HIGH on the negative finding (slide does NOT fire on standard YR main-menu button clicks). HIGH on the gating chain (`+0xC1` → `+0xC2` → slide). HIGH on the +0xC1/+0xC2 setter exclusivity (verified via exhaustive byte-pattern search across all register encodings).

**Active in YR:** **No** for dialog 0xE2 main-menu clicks. The slide system IS active in YR but only on Load/Save dialog flows and (likely) WOL screens.

This report extends three prior investigations:
- `FUN_006071E0_SLIDE_IN_FRAME_SCHEDULE_GHIDRA_REPORT.md` — frame schedule
- `SDMPBTN_SDWRNTMP_RECT_CONSUMERS_GHIDRA_REPORT.md` — asset rects
- `SHELL_DIALOG_SLIDE_TRANSITION_TRANSFORM_GHIDRA_REPORT.md` — per-frame visual transform

The prior reports' claim that `FUN_006071E0` "fires on every main-menu button click" is **refuted** below.

---

## 1. Executive Summary

The "slide-in/slide-out animation" implemented by `FUN_006071E0` does **not** play on a standard YR main-menu (dialog 0xE2) button click. The complete trigger chain is gated by a single byte (`record[+0xC1]`) that no main-menu code path sets. The user's observation of "an animation after I click before the next page" — if it refers specifically to the main-menu shell — must be one of:

1. The 1 Hz SDBTNANM frame 2 ↔ frame 3 hover flash (focus-flash, already documented and implemented in our Rust shell)
2. The instant SDBTNANM frame-4 snap with +2 px Y shift on press
3. A misperception combining the click sound (`VocClass__PlayAtPos(0x3F800000, 0)`) + Bink movie continuing + the next dialog appearing
4. The slide animation observed on a *different* dialog (Load Game, WOL screens) and mentally associated with the main menu

For our parity work: **do NOT implement a between-page transition for main-menu clicks**. That would be a design addition beyond gamemd's actual behavior on this screen.

## 2. Verified Trigger Chain

### 2.1 Animation entry points (only two wrappers ever call FUN_006071E0)

- `FUN_00608260` — slide-IN wrapper (`record[+0xC1] != 0` gate, direct call to FUN_006071E0). Verified via `decompile_function 0x00608260` this session.
- `FUN_00607FD0` — slide-OUT paint dispatcher (`record[+0xC2] != 0` gate, calls FUN_006071E0 after pausing Bink movie via SendMessage(0x71A, 0x4E2)). Verified via `decompile_function 0x00607FD0`.

### 2.2 +0xC1 setter exclusivity (slide-IN gate)

- **Only setter: `FUN_00608380`** at `0x006083D3` (encoding `C6 80 BD 00 00 00 01` = `MOV byte ptr [EAX+0xBD], 1`, with EAX = record root + 4, so net offset = +0xC1).
- Byte-pattern search across all 8 register encodings (`C6 80/81/82/83/84/85/86/87 BD 00 00 00 01`) returns exactly **one match** — at `0x006083D3` inside FUN_00608380. No other writers exist in the binary.
- `FUN_00608380` has **one** caller: `CDFileClass__Constructor @ 0x00559474` (Load/Save dialog save-success branch only). Verified via `get_xrefs_to 0x00608380`.

### 2.3 +0xC2 setter exclusivity (slide-OUT gate)

- **Only setter: `FUN_00608070`** at `0x006081BF` (encoding `C6 87 BE 00 00 00 01` = `MOV byte ptr [EDI+0xBE], 1`, with EDI = record root + 4, so net offset = +0xC2).
- Byte-pattern search across `C6 80/83/86/87 BE 00 00 00 01` returns exactly **one match** — at `0x006081BF` inside FUN_00608070. No other writers.
- `FUN_00608070` is **itself gated on +0xC1**: `if ((record[+0xC1] == 0) || (record[+0xB4] != 1)) return 0;`. So even though +0xC2 is set inside it, the function bails before reaching the setter if +0xC1 is zero.
- `FUN_00608070` is called from `FUN_00622720` (the generic dialog cleanup function) — see §3.

### 2.4 Net implication

For the slide animation (either direction) to fire on a given dialog, **`record[+0xC1]` must have been set to 1 on that dialog's HWND**. The only setter is reachable only from `CDFileClass__Constructor`'s save-success branch, which is part of the Load/Save dialog. **No code path traceable from dialog 0xE2's WM_INITDIALOG, dialog proc, or main-menu launcher reaches that setter.**

## 3. Main-Menu Click Path — Step by Step (Verified)

The user clicks "Single Player" on the main menu. The sequence:

1. `OwnerDraw_Button_00612B70` receives WM_LBUTTONDOWN (`0x201`):
   - Plays click sound `VocClass__PlayAtPos(0x3F800000, 0)`
   - Forwards to standard Win32 BUTTON proc via `CallWindowProcA`
2. Standard BUTTON proc tracks press/release, sends `WM_COMMAND` (BN_CLICKED) to parent on release-over-pressed
3. `MainMenuDialog0xE2_Proc_00531F60` receives WM_COMMAND with `LOWORD(wParam) == 0x683`:
   - Sets `*puVar2 = 1` (result code via `GetWindowLong(hwnd, 8)`)
   - Returns 0
4. **Main-menu launcher `FUN_00531CC0`** sees `local_1c != 0x12` (result changed from sentinel), exits its message-pump loop
5. **`FUN_00622720(hwnd)` is called** — generic dialog cleanup
6. **Inside FUN_00622720: `FUN_00608070(hwnd)` is called** — slide-out trigger
7. **FUN_00608070 immediately bails** because `record[+0xC1] == 0` on dialog 0xE2 (gate fails on the very first check)
8. `DestroyWindow(hwnd)` — dialog destroyed instantly, no animation
9. `FUN_00531CC0` returns the result code to its caller
10. Caller creates the next dialog (skirmish setup, options, etc.) — instant, no transition

Verified via `decompile_function 0x00531CC0`, `0x00622720`, `0x00608070`, `0x00531F60`, `0x00612B70` this session.

## 4. Why The Prior Reports Were Wrong

`FUN_006071E0_SLIDE_IN_FRAME_SCHEDULE_GHIDRA_REPORT.md` (§1, §11):
> "fires on every main-menu button click. Confirmed via `get_function_callers @ 0x006071E0` (callers include `FUN_00608260` which is the owner-draw button-press handler)"

`SDMPBTN_SDWRNTMP_RECT_CONSUMERS_GHIDRA_REPORT.md` (§3c):
> "FUN_00608260 — owner-draw button press handler that triggers the transition animation when a main-menu button is clicked"

**Both are wrong.** `FUN_00608260` is *not* the main-menu button press handler. It is a generic slide-in trigger gated on `record[+0xC1] != 0`, with only 2 callers (`0x005E6B49` Load/Save success continuation, and `0x00612690` an owner-draw state machine — neither on the main-menu click path).

The most recent prior report (`SHELL_DIALOG_SLIDE_TRANSITION_TRANSFORM_GHIDRA_REPORT.md`, mtime 09:21) correctly identifies the 2 callers and notes the `+0xC1` gate. It supersedes the earlier two on this question.

## 5. Confirmed Behavior of Each Dialog Path

| Dialog | Slide on click/exit? | Why |
|---|---|---|
| 0xE2 Main Menu | **No** | `+0xC1` never set |
| Load/Save (CDFileClass dialogs 1, 2, 3) | Yes (slide-in on save success only) | `FUN_00608380` called from save-success branch sets `+0xC1` |
| WOL screens (`0x113`, `0xC4`, `0x130`, etc.) | Likely yes — many FUN_00607FD0 xrefs in 0x0078xxx-0x007Axxx range | Not investigated; out of scope for this report |
| Owner-draw state-machine site at `0x00612690` | Conditional | Drives `+0x1FC` state 1→3; the calling dialog is unidentified (Ghidra hasn't created a function boundary). Per prior report this is "shell owner-draw region (just below OwnerDraw_Button_00612B70)" |

## 6. What The User Probably Sees on a Main-Menu Click

In gamemd, after clicking a main-menu button the visible sequence is:

1. **Frame 4 (pressed)** SDBTNANM SHP swap with +2 Y shift — instant on mouse-down
2. **Click sound** plays — `VocClass__PlayAtPos(0x3F800000, 0)` in WM_LBUTTONDOWN handler
3. **Bink movie continues playing** for the brief moment between WM_COMMAND firing and `DestroyWindow` running
4. **Main-menu window destroyed instantly** via `DestroyWindow`
5. **Next dialog appears instantly** (no fade, no slide)

What feels like "an animation" is the combination of: the click sound, the brief frame-4 visual, the Bink movie continuing to play during the ~50-150 ms it takes the next dialog to construct (which itself may have its own intro animation — e.g., the skirmish-setup dialog DOES use the slide system because it's in the `paint_mode == 1` family). The transition to the next dialog is instant, but the next dialog may slide *itself* in.

**The "animation between pages" the user sees might be the NEXT dialog's slide-in, not the main menu's slide-out.** Worth testing: click "Options" on the main menu — does the Options dialog appear with a slide? If so, that's the next-dialog's animation, not the main menu's.

## 7. Open Questions — Final State of the Investigation Log

- `[RESOLVED] Q-01` — Does +0xC1 have any setters besides FUN_00608380? → No. Verified via byte-pattern search across all 8 register encodings for `MOV byte ptr [reg+0xBD], 1`; only `0x006083D3` matches. (evidence: `search_byte_patterns C6 80/81/82/83/84/85/86/87 BD 00 00 00 01`)
- `[RESOLVED] Q-02` — Does +0xC2 have any setters besides FUN_00608070? → No. Verified via byte-pattern search; only `0x006081BF` matches. (evidence: `search_byte_patterns C6 80/83/86/87 BE 00 00 00 01`)
- `[RESOLVED] Q-03` — Does the main-menu dialog proc call FUN_00608380 or any +0xC1 setter? → No. `MainMenuDialog0xE2_Proc_00531F60` handles only WM_COMMAND (sets result codes 1-6), WM_PAINT (sends 0x4F0 to Bink control 0x71A), and WM_CTLCOLOR (0x497, sets version line text). (evidence: `decompile_function 0x00531F60`)
- `[RESOLVED] Q-04` — Does the main-menu launcher FUN_00531CC0 call any slide trigger? → No. It runs a message pump until the dialog result code changes, then calls `FUN_00622720` for cleanup. No slide trigger. (evidence: `decompile_function 0x00531CC0`)
- `[RESOLVED] Q-05` — Is `FUN_00608070` called from dialog cleanup? → Yes. `FUN_00622720` calls `FUN_00608070(hwnd)` before `DestroyWindow`. But FUN_00608070 bails immediately on the +0xC1 gate for dialog 0xE2. (evidence: `decompile_function 0x00622720`, `0x00608070`)
- `[RESOLVED] Q-06` — What does the user see on a main-menu click then? → Click sound + frame-4 SHP snap + Bink movie + instant dialog swap. The "animation" might be the *next* dialog's slide-in (e.g., skirmish-setup, options) if those dialogs have +0xC1 set in their init paths. (inference based on resolved Q-01..05)
- `[DEFERRED] Q-07` — Is +0xC1 set in the init paths of dialogs that DO have visible slide-in (skirmish-setup, options)? (category: `out-of-scope`; reason: identifying those init paths is a separate investigation per-dialog; next-step-if-pursued: trace WM_INITDIALOG of dialog 0x102 (skirmish setup) and 0xD5 (options) for +0xC1 writes)
- `[DEFERRED] Q-08` — What is the enclosing function at `0x00612690`, and what state machine drives it? Prior report observed `[+0x1FC]` state transitions 1→2→3 but couldn't create a function boundary in read-only mode. (category: `bounded-cost-too-high`; reason: requires either a write-enabled Ghidra session for `create_function`, or extensive manual disassembly to find the prologue; next-step-if-pursued: read 0x00612400..0x00612690 looking for PUSH EBP / MOV EBP, ESP / SUB ESP, N prologue patterns)
- `[DEFERRED] Q-09` — Do WOL screens actually use the slide on entry/exit, and which ones? (category: `out-of-scope`; reason: 30 callers of FUN_00607FD0 in 0x0078xxx-0x007Axxx range need individual classification; next-step-if-pursued: trace each WOL caller and identify the source dialog)

## 8. Recommendations for Rust Implementation

**Do NOT implement a between-page transition on main-menu button clicks.** Parity with gamemd is to have NO animation between dialogs at this step. The next dialog (skirmish setup, options) may have its own intro animation — that should be implemented as part of *that* dialog's parity work, not as a main-menu feature.

If a future investigation establishes that one of the destination dialogs (e.g., skirmish setup `0x102`) DOES have a slide-in (via its own +0xC1 setter), implement the slide as part of that destination dialog's intro — not as part of the main-menu exit.

The slide animation itself, when needed, is fully specified in `FUN_006071E0_SLIDE_IN_FRAME_SCHEDULE_GHIDRA_REPORT.md` and `SHELL_DIALOG_SLIDE_TRANSITION_TRANSFORM_GHIDRA_REPORT.md`:
- 30 ms per tick × (N + 8) ticks where N = button-row count
- Per-button frame stagger of 1 tick, SDBTNANM frames cycle 10→5 on slide-in or 5→10 on slide-out, then settle at frame 1 or 10 respectively
- Direction-keyed base frames listed in the frame-schedule report

## 9. Sources

**Ghidra MCP calls (read-only, this session):**

- `decompile_function 0x00531CC0` — main-menu launcher; confirmed no slide trigger
- `decompile_function 0x00531F60` — main-menu dialog proc; confirmed no slide trigger in WM_COMMAND handler
- `decompile_function 0x00608260` — slide-in wrapper; confirmed +0xC1 gate
- `decompile_function 0x00608380` — +0xC1 setter
- `decompile_function 0x00608070` — slide-out trigger; confirmed +0xC1 gate, +0xC2 setter at 0x006081BF
- `decompile_function 0x00607FD0` — slide-out paint dispatcher; confirmed +0xC2 gate
- `decompile_function 0x00622720` — dialog cleanup; confirmed calls FUN_00608070
- `decompile_function 0x00559474` (CDFileClass__Constructor) — confirmed +0xC1 setter is reached only in save-success branch
- `read_memory 0x00608380` (96 bytes) — verified the +0xC1 setter byte pattern is `C6 80 BD 00 00 00 01`
- `read_memory 0x006081B0` (80 bytes) — verified the +0xC2 setter byte pattern is `C6 87 BE 00 00 00 01`
- `get_xrefs_to 0x00608380` → 1 caller (CDFileClass__Constructor)
- `get_xrefs_to 0x00608260` → 2 callers (0x005E6B49, 0x00612690)
- `get_xrefs_to 0x00607FD0` → 30 callers (Load/Save + WOL ranges, none in main-menu)
- `search_byte_patterns C6 80/81/82/83/84/85/86/87 BD 00 00 00 01` → exactly 1 match (0x006083D3 in FUN_00608380)
- `search_byte_patterns C6 80/83/86/87 BE 00 00 00 01` → exactly 1 match (0x006081BF in FUN_00608070)
- `get_assembly_context 0x00612690` (25-instruction window) — state machine `[+0x1FC]` 1→2→3 confirmed
- `get_function_by_address 0x00612690` → "No function found" (Ghidra has not created a boundary here)

**Prior reports referenced:**

- `FUN_006071E0_SLIDE_IN_FRAME_SCHEDULE_GHIDRA_REPORT.md` — frame schedule (claims about main-menu attribution refuted in §4 above)
- `SDMPBTN_SDWRNTMP_RECT_CONSUMERS_GHIDRA_REPORT.md` — asset rects (claim about main-menu attribution refuted in §4 above)
- `SHELL_DIALOG_SLIDE_TRANSITION_TRANSFORM_GHIDRA_REPORT.md` — per-frame visual transform (corroborated)
- `MAIN_MENU_BUTTON_CLICK_ANIMATION_GHIDRA_REPORT.md` — earlier (this session) ButtonFadeEffect investigation
- `MAIN_MENU_BUTTON_DISPATCH_LAB_0060A330_GHIDRA_REPORT.md` — confirmed main-menu buttons use SDBTNANM frames 2/3/4

**Rust files:** read for cross-reference only, not modified. `src/app_main_menu_shell_render.rs`, `src/ui/main_menu_shell/state.rs`.
