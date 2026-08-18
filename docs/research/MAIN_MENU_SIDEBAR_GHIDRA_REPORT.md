# Main Menu Sidebar / Initial Shell Menu - Ghidra Report

Date: 2026-05-17

Scope: standard Yuri's Revenge initial main menu, especially the right-side button
stack and the large left visual panel. This is the screen reached from
`Main_Game` when the shell state is `0x12`.

## Executive summary

The initial main menu is a Win32 shell dialog, not the in-game `SidebarClass`.
It is also not driven by the `GraphicMenu`/`Title.PCX` path for standard YR
startup.

Verified live path:

1. `Main_Game` (`0x0052D9A0`) enters shell state `0x12`.
2. It calls `FUN_00531CC0`.
3. `FUN_00531CC0` creates RT_DIALOG resource `0xE2` with dialog proc
   `0x00531F60`.
4. Common shell setup subclasses dialog controls through `FUN_0060F9A0`.
5. The six right-column buttons are normal shell owner-draw buttons using
   `OwnerDraw_Button_00612B70`.
6. Child static `0x71A` is subclassed to `OwnerDraw_Static_006153E0` and is
   commanded to play looping movie base name `Ra2ts_s` at 640-wide mode,
   otherwise `Ra2ts_l`. Follow-up asset work confirmed the generic
   VQ-named movie handle resolves `.BIK` first and `.VQA` second, so retail YR
   uses Bink-backed `Ra2ts_*` files when present.

Player-visible implication: the "main menu sidebar" is really a right-side
stack of owner-draw shell buttons over a Win32 dialog, with a separate looping
movie/static panel on the left. It should not be implemented by reusing the
in-game build sidebar or by assuming the Skirmish dialog `0x102` is the same
screen.

## Binary entry points

### Main menu creation

`FUN_00531CC0` begins with:

```text
00531CC4  OR ECX,0xffffffff
00531CC7  MOV [ESP+8],0x12
00531CCF  CALL 0x004790B0
00531CD4  MOV EDX,0x531F60
00531CD9  MOV ECX,0xE2
00531CDE  PUSH 0
00531CE0  CALL 0x00622650
```

Verified meaning:

- resource/dialog id: `0xE2`
- dialog proc: `0x00531F60`
- initial loop result: `0x12` means stay on the main menu
- `FUN_00622650` loads a dialog template and calls
  `CreateDialogIndirectParamA`

`FUN_00622650` stores the created `HWND` and dialog id in the global shell
dialog stack (`DAT_00B72D28/2C`, `DAT_00B72F44/48`) after creation succeeds.

### RT_DIALOG `0xE2`

The dialog template is `DIALOGEX`, style `0x40000040`, rect `0,0,533,369`,
font `MS Sans Serif`, 8 pt. Coordinates below are dialog-template units; Win32
performs the DLU-to-pixel conversion.

| Control id | Class | Rect | Style | Title |
|---:|---|---|---:|---|
| `0x3EE` | Button | `425,330,108,23` | `0x5000000B` | `GUI:ExitGame` |
| `0x683` | Button | `425,125,108,23` | `0x5000000B` | `GUI:SinglePlayer` |
| `0x684` | Button | `425,152,108,23` | `0x5000000B` | `GUI:WWOnline` |
| `0x55C` | Button | `425,233,108,23` | `0x5000000B` | `GUI:Options` |
| `0x578` | Button | `425,179,108,23` | `0x5000000B` | `GUI:Network` |
| `0x694` | Static | `425,1,108,10` | `0x50020001` | `GUI:MainMenu` |
| `0x695` | Static | `2,355,303,12` | `0x50000200` | `GUI:Blank` |
| `0x686` | Button | `425,206,108,23` | `0x5000000B` | `GUI:MoviesAndCredits` |
| `0x71A` | Static | `0,0,304,266` | `0x50000000` | none |
| `0x71C` | Static | `447,29,61,33` | `0x50000007` | none |
| `0x71D` | Static | `425,357,108,10` | `0x50000001` | `GUI:Blank` |

The visible interactive right-side menu entries are the six `Button` controls.
The large left-area `0x71A` static is the movie panel, not the button sidebar.

## Dialog proc behavior

The dialog proc at `0x00531F60` first lets common shell code handle the message:

```text
00531F84  MOV EDX,EDI
00531F86  MOV ECX,ESI
00531F88  CALL 0x00622B50
00531F8D  TEST EAX,EAX
00531F8F  JNZ 0x005320F3
```

Then it handles three cases directly.

### `WM_COMMAND` button ids

The proc obtains the result pointer from `GetWindowLong(hwnd, 8)` and writes a
menu return code when a button is clicked:

| Button id | Label | Return code |
|---:|---|---:|
| `0x683` | Single Player | `1` |
| `0x684` | WW Online | `2` |
| `0x578` | Network | `3` |
| `0x686` | Movies and Credits | `4` |
| `0x55C` | Options | `5` |
| `0x3EE` | Exit Game | `6` |

The menu loop in `FUN_00531CC0` exits once the result is no longer `0x12`.

### `WM_PAINT`

For `WM_PAINT`, the proc gets child `0x71A` and sends custom message `0x4F0`:

```text
005320D5  PUSH 0x71A
005320DA  PUSH ESI
005320DB  CALL GetDlgItem
005320E1  PUSH 0
005320E3  PUSH 0
005320E5  PUSH 0x4F0
005320EA  PUSH EAX
005320EB  CALL SendMessageA
```

`0x4F0` is handled by `OwnerDraw_Static_006153E0` as the explicit movie draw or
frame blit path when a movie handle exists.

### Custom `0x497`

The proc handles `0x497` by writing status text into static `0x71D`. This is
shell status text plumbing and not the main button dispatch path.

## Left panel / movie behavior

After the dialog is created and shown, `FUN_00531CC0` finds child `0x71A` and
positions it:

```text
x = 0 if screen_width  <= 800 else (screen_width  - 800) / 2
y = 0 if screen_height <= 600 else (screen_height - 600) / 2
SetWindowPos(child_0x71A, 0, x, y, -1, -1, 0xD)
```

Then it sends:

```text
SendMessage(0x71A, 0x4E3, 1, 0)
SendMessage(0x71A, 0x4E4, 0, screen_width == 640 ? "Ra2ts_s" : "Ra2ts_l")
```

`FUN_0052B9B0` duplicates the same reposition and `0x4E3`/`0x4E4` sequence,
so this is also the refresh/reposition helper.

### Static subclass

`FUN_0060F9A0` maps Win32 class `Static` to `OwnerDraw_Static_006153E0` with
control type `0x2`. Therefore child `0x71A` is a normal dialog static whose
window proc has been replaced by the shell owner-draw static proc.

`OwnerDraw_Static_006153E0` verifies the custom messages:

| Message | Verified behavior |
|---:|---|
| `0x4E2` | Destroys current movie handle, kills timer `0x65`, clears fallback surface state |
| `0x4E3` | Stores `wParam` as a loop flag; main menu passes `1` |
| `0x4E4` | Clears prior movie/surface state and creates a generic movie handle using the provided asset base name; follow-up decompilation shows `.BIK` is tried before `.VQA` |
| `0x4F0` | Calls the movie object's draw/update method when a movie handle exists |
| `WM_TIMER`, timer `0x65` | Advances movie frames every `0x22` ms and invalidates changed frames |

When the movie reports end-of-playback, the stored `0x4E3` loop flag controls
whether the movie loops. Main menu sets the flag, so `Ra2ts_s/l` loops.

The decompiler is imperfect around the `0x4E4` constructor call, but the message
handler receives the name pointer in `lParam`, constructs/owns a generic movie
handle, uses movie vtable methods for size/draw/advance/end checks, and sets a
timer. Follow-up decompilation of the constructor (`0x005C07D0`) shows the live
retail path is Bink when `Ra2ts_*.BIK` resolves, with legacy VQA fallback only
if no Bink file is found. That is enough to classify `Ra2ts_s` and `Ra2ts_l` as
movie asset base names in this live path, not INI section names for
`GraphicMenu`.

## Right-side button behavior

`FUN_0060F9A0` maps Win32 class `Button` with style bits matching `0x0B` to
`OwnerDraw_Button_00612B70`. All six main menu buttons in resource `0xE2` have
style `0x5000000B`, so they use this shared shell owner-draw button renderer.

The button proc verifies:

- `WM_LBUTTONDOWN` and `WM_LBUTTONDBLCLK` play the global main GUI button sound
  unless the control is disabled.
- The sound is loaded from `RulesClass + 0x188`, which prior sound research maps
  to `[AudioVisual] GUIMainButtonSound`.
- Default `rulesmd.ini` and `rules.ini` set `GUIMainButtonSound=MenuClick`.
- Owner-draw visual pieces are generated from format strings:
  - `b%c%c_li%d.pcx`
  - `b%c%c_mi%d.pcx`
  - `b%c%c_ri%d.pcx`
- The state character switches between unpressed/up and down/pressed state
  (`'u'` / `'d'` in the assembly), and the second character is literal `'e'`.

Important distinction: these are generic shell owner-draw buttons. They are
shared with other shell dialogs and are not `SidebarClass` or `ShapeButtonClass`.

## Tooltips / string table

`FUN_006040B0` returns tooltip string ids by dialog id and control id. For
dialog `0xE2`:

| Control id | Tooltip string |
|---:|---|
| `0x683` | `STT:MainButtonSinglePlayer` |
| `0x684` | `STT:MainButtonWWOnline` |
| `0x578` | `STT:MainButtonNetwork` |
| `0x686` | `STT:MainButtonMovies` |
| `0x55C` | `STT:MainButtonOptions` |
| `0x3EE` | `STT:MainButtonExitGamemd` |
| `0x55F` | `STT:MainButtonYuriWebSite` |

The `STT:MainButton*` strings are tooltip identifiers, not proof of a separate
graphic-menu item model.

## Main_Game dispatch

`Main_Game` calls `FUN_00531CC0` when the shell state is `0x12` and the relevant
skip flag is not set. It then switches on the returned menu code:

| Return code | Main_Game behavior |
|---:|---|
| `1` | Clears `DAT_00AC10C8`, calls `FUN_0060D380(1)` for single-player shell path |
| `2` | Sets `g_GameMode = 4`, calls `FUN_0053F1F0()` for WOL/online |
| `3` | Sets `g_GameMode = 3`, calls `FUN_0053F1F0()` for LAN/network |
| `4` | Calls `FUN_0060D380(1)` for movies/credits path |
| `5` | Calls `OptionsClass__ShowLauncherDialog()`, then returns to state `0x12` |
| `6` | Loads strings `0x9C6/0x9C7/0x9C8`, shows exit confirmation, writes options on confirmed exit |
| `7` | Exit/quit path |

This confirms the right-side button ids map directly to shell routing codes.

## Sounds and INI keys

Verified in binary and local INI:

| Key | Default | Verified role |
|---|---|---|
| `GUIMainButtonSound` | `MenuClick` | Played by `OwnerDraw_Button_00612B70` on main shell button mouse down/double-click through `RulesClass + 0x188` |
| `ShellButtonSlideSound` | empty | Parsed by the binary and present in prior sound research, but no active trigger was found in the initial menu path during this pass |

The investigation did not find item-local `GraphicMenu` keys (`Image`,
`Highlighted`, `Disabled`, `Origin`, `ActiveRect`, `SelectSound`, `SelectVQ`) in
the live standard main menu path. Those keys belong to the rejected
`GraphicMenu` hypothesis unless another caller is later proven live.

## `GraphicMenu` / `Title.PCX` status

The plan suspected `GraphicMenu` and `Title.PCX`. Phase 1 disproved that for the
standard initial menu:

- `FUN_00531CC0` creates Win32 dialog resource `0xE2`.
- The visible main menu buttons are dialog controls.
- The left panel is static control `0x71A` with a movie owner-draw proc.
- `Ra2ts_s/l` are passed as custom-message payloads to the static owner-draw
  proc, not parsed as INI sections by `GraphicMenu`. The generic movie
  constructor tries `.BIK` before `.VQA`.
- No `GraphicMenu` constructor is needed to explain the live main-menu render,
  input, sound, or return-code path.

`GraphicMenu` may still be legacy shell code or used by another screen. It
should not be treated as the behavioral source for this main menu without a new
caller proof.

## Current Rust status

Observed local Rust implementation:

- `src/ui/main_menu.rs` is an egui skirmish setup/menu surface. Its module
  comment explicitly says it is pragmatic client shell UI rather than
  pixel-perfect RA2 chrome.
- `src/app.rs` renders `GameScreen::MainMenu` either through the egui menu or,
  when `RA2_DEV_SKIRMISH_SHELL` is enabled, the dev Skirmish shell renderer.
- `src/app_skirmish_shell_render.rs` and `src/render/skirmish_shell_chrome.rs`
  are Skirmish-dialog paths, adjacent but not evidence for initial dialog `0xE2`.

Parity gap: the Rust main menu currently does not implement dialog `0xE2`, the
right-side shell owner-draw button stack, the looping `Ra2ts_s/l` Bink/movie
panel, the `GUIMainButtonSound` trigger for these buttons, or the exact
return-code routing.

### 2026-07-25 current-Rust correction

That parity-gap paragraph described the earlier checkout. Production `dev` at
`e726da11` now implements the dedicated `0xE2` screen, owner-draw button stack,
retail `RA2TS_S`/`RA2TS_L` movie panel, button return-code routing, and the
intermediate `0x100` Single Player route. Those mechanisms have focused
regression coverage but no comparable full-frame native differential.

Exact button pixels, Bink frame/timer phase, cursor presentation, transition
frames, music/UI-sound ordering, and the aggregate route remain `UNVERIFIED`.
The same-event-loop no-paint lifecycle defect documented in
`MAIN_MENU_RA2TS_PLAYBACK_ARCHIVE_PRIORITY_GHIDRA_REPORT.md` is corrected by
reviewed feature commit `3a96251e` but was not yet in `dev`; do not use code
presence or the focused Rust tests as exact-parity evidence.

## Implementation implications

For a faithful replacement, model this as a shell/menu layer above `sim/`:

- Do not reuse `SidebarClass` research for this screen.
- Do not wire this through the Skirmish dialog `0x102` renderer.
- Reuse shell owner-draw button knowledge where useful: segmented PCX button
  pieces, GAME.FNT text, disabled/pressed state, and `GUIMainButtonSound`.
- Add a main-menu visual panel capable of playing `Ra2ts_s` at 640-wide mode and
  `Ra2ts_l` otherwise. Retail YR resolves these to `.BIK` first; the owner-draw
  static still uses a 34 ms timer around the movie handle.
- Preserve the menu return-code contract because `Main_Game` routing depends on
  those exact codes.

## Open questions

These were not required to establish the live path but should be resolved before
pixel-perfect implementation:

1. Exact archive priority for duplicate `Ra2ts_s/l.BIK` files between base RA2
   and YR MIX archives before final pixel-diff implementation.
2. Exact DLU-to-pixel conversion after font metrics and any shell layout helper
   adjustments for dialog `0xE2`.
3. Complete button PCX asset inventory for the `bue_*` / `bde_*` shell button
   pieces in retail MIX files.
4. Whether `ShellButtonSlideSound` is triggered by another shell animation path
   and merely silent by default, or unused in retail YR initial menu.
5. Whether control `0x55F` (`STT:MainButtonYuriWebSite`) is present in another
   variant of dialog `0xE2` or a related WOL/main-menu resource.

## Evidence list

Primary binary functions inspected:

- `Main_Game` `0x0052D9A0`
- `FUN_00531CC0` main menu loop
- dialog proc entry `0x00531F60`
- `FUN_0052B9B0` main visual panel refresh/reposition helper
- `FUN_00622650` shell dialog creation
- `FUN_00622B50` common shell dialog proc
- `FUN_0060F9A0` owner-draw subclass setup
- `OwnerDraw_Static_006153E0`
- `OwnerDraw_Button_00612B70`
- `FUN_006040B0` tooltip string registry

Local data checked:

- RT_DIALOG resource `0xE2` parsed from `gamemd.exe`
- `ini/rulesmd.ini`
- `ini/rules.ini`
- `src/ui/main_menu.rs`
- `src/app.rs`
- `src/app_skirmish_shell_render.rs`
- prior reports in `C:/Users/enok/Documents/ra2-rust-game-docs/`, especially
  global/sound and shell owner-draw research

## Related reports (added 2026-05-18 main-menu --area swarm)

The 2026-05-18 main-menu swarm produced five new reports that extend or
partially resolve open questions in this doc:

- `MAIN_MENU_MUSIC_TRACK_AND_LOOP_GHIDRA_REPORT.md` — shell music is the
  `[INTRO]` theme (`Sound=Drok, Repeat=yes`); started from
  `Main_Game @ 0x0052D9A0`, looped by `Theme::AI` polling.
- `SHELL_BUTTON_SLIDE_SOUND_CALL_SITE_GHIDRA_REPORT.md` — **resolves open
  question #4 (ShellButtonSlideSound).** Single live consumer at
  `0x00607F59` (`RulesClass + 0x750`); fires on slide-in *open* direction
  only (Load Game success path). Initial main menu dialog `0xE2` does NOT
  trigger it. Silent by default because shipped INI is empty.
- `EVA_WELCOME_BACK_MAIN_MENU_TRIGGER_GHIDRA_REPORT.md` — verified-negative:
  no "Welcome back, Commander" or similar EVA cue exists on shell entry.
  Only audio on entry is the INTRO theme.
- `QUIT_CONFIRM_DIALOG_MAIN_MENU_GHIDRA_REPORT.md` — Quit button (control
  `0x3EE`) → `Main_Game` case-6 → CSF-driven modal `FUN_005D3490` with
  RT_DIALOG `0x120`, strings `GUI:ExitAreYouSure / TXT_OK / GUI:Cancel`.
  Confirm path: case-7 → `OptionsClass::WriteToINI` → music stop → fade →
  return-cascade out of WinMain. No `PostQuitMessage` / `ExitProcess`.
- `SDBTNANM_FRAME10_OVERLAY_CONDITION_GHIDRA_REPORT.md` — partial: frame-10
  is a binary highlight-vs-default selector, predicate at WindowExtra
  record `+0xD8`. Sticky-on (no live clearer in current xrefs); UX
  semantic name not determined from binary alone.
