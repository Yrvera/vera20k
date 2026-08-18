# Main-Menu Cursor: SHP, Loader, and Hover Rules - Ghidra Report

Date: 2026-05-19

Scope (one-line): When the main-menu dialog `0xE2` is up, which cursor asset is
the player seeing? Where is it loaded? Does the engine draw it via SHP, and are
there per-control hover overrides?

Active in YR: Yes. Verified in standard YR shell path.

## Executive Summary

The cursor visible over the main menu dialog `0xE2` is **the engine's own
software-blitted SHP cursor**, not a Win32 `IDC_ARROW`. The OS cursor is hidden
for the entire lifetime of the process. The SHP asset is `MOUSE.SHA`, loaded
from the MIX archives at boot during the asset-preload pass (well before
the shell dialog is created). The cursor stays on **MouseClass cursor ID 0
(frame 0, the default arrow)** for the entire shell screen lifetime: no
WM_SETCURSOR / LoadCursorA / SetCursor calls fire from the main-menu dialog
proc, the common shell dialog proc, or the owner-draw button procedure.

There is **no per-control hover cursor change** anywhere in the `0xE2` paint
or input path: the visual button hover state (`+0xC5` flag) drives a sprite
swap inside `OwnerDraw_Button_00612B70`, but never alters the cursor shape.

Confidence:
- **HIGH** (content + identity): cursor file name `MOUSE.SHA`,
  loader chain (`MouseClass::One_Time -> CDFileClass`), per-frame WWMouseClass
  blit, and the absence of WM_SETCURSOR in the dialog stack. All read directly
  from the binary in this session.
- **MEDIUM** (binding for the WWMouseClass vtable[1] decode): cursor blit
  surface is read out of the vtable but slot-by-slot WWMouse method names are
  inferred from RA2/TS legacy mouse architecture, not labeled in Ghidra.

## What the player sees

A single static cursor sprite: frame 0 of `MOUSE.SHA`, hotspot (0,0), no
animation, no hover variants. It is drawn by the engine into the back surface
each frame just before flip; the Win32 cursor under it has been hidden by
`ShowCursor(0)` at WWMouseClass construction.

## Loader and asset path

### Window-class cursor is irrelevant for the visible cursor

`CreateMainWindow @ 0x00777C30` registers the window class with
`hCursor = LoadCursorA(hInstance, MAKEINTRESOURCE(0x68))` and also calls
`SetCursor(LoadCursorA(hInstance, 0x68))` once after window creation. This
sets the Win32 default for the `"Yuri's Revenge"` window class, but the OS
cursor is hidden almost immediately afterward (see next), so the player
never actually sees the Win32 cursor. The exact bitmap behind resource
`0x68` is **unknown** from this session (it lives in the PE resource tree
and was not extracted).

### OS cursor hidden by WWMouseClass

`WWMouseClass::Constructor @ 0x007B8730` calls `ShowCursor(0)` at the end of
construction. `WWMouseClass` is constructed once in `WinMain @ 0x006BB9A0`
before `Main_Game()` is entered, so the OS cursor stays hidden for the entire
process lifetime - including the shell screen.

### MOUSE.SHA load path

The shell cursor SHP is loaded inside `MouseClass::One_Time @ 0x005BDF30`:

```c
DAT_00abf294 = CDFileClass__Constructor();  // arg = "MOUSE.SHA" (0x0082604C)
```

The filename string `MOUSE.SHA` lives at `0x0082604C`. The resulting SHP-data
pointer is stored at the global `0x00ABF294`, the same global referenced
by `MouseClass::SetMouseShape @ 0x005BDC80` when blitting the cursor frame.

`MouseClass::One_Time` is called from `CCFileClass__Constructor @ 0x00531680`,
which is the boot-time asset preload pass (run after MIX archives mount, before
any dialog is created). So `MOUSE.SHA` is in memory before the main menu
dialog `0xE2` first appears.

Note: the file extension is **`.SHA`, not `.SHP`** despite "MOUSE.SHP" being a
common label in mod docs. No `MOUSE.SHP` or `mouse.shp` string exists anywhere
in `gamemd.exe` (verified by string search).

### Cursor ID 0 (default arrow) selected at boot

`MouseClass::SetCursor(0, 0) @ 0x005BDA80` is invoked from
`CCFileClass__Constructor @ 0x0052BA60` during the same boot-preload pass.
The cursor data table at `0x0082D028` entry 0 is:

| field         | value |
|---------------|------:|
| StartFrame    | 0     |
| FrameCount    | 1     |
| MiniStartFrame| 0     |
| MiniFrameCount| 1     |
| FrameRate     | 1     |
| HotSpotX      | 0     |
| HotSpotY      | 0     |

i.e. single-frame, no animation, top-left hotspot - the default arrow.

### Per-frame draw site

`FUN_004F4780` is the back-buffer flush. It calls:

- `g_DisplayChain->vtable[0x3C]` (Hide_Mouse - restore pixels under cursor),
- the surface blit,
- `g_DisplayChain->vtable[0x40]` (Show_Mouse - blit cursor SHP into surface).

Inside the shell pump (`FUN_00532100 -> FUN_004F4780`), this path runs every
time the shell decides to redraw. The cursor sprite drawn is whatever
WWMouseClass currently holds via `vtable[1]` at `0x007B8A00`, which was last
set by `MouseClass::SetMouseShape` using `MOUSE.SHA` frame 0.

## Rules that change cursor on hover/click in dialog 0xE2

**There are none.** The chain `dialog proc 0x00531F60 -> common dialog proc
FUN_00622B50 -> owner-draw button proc OwnerDraw_Button_00612B70` was audited
end-to-end:

- `MainMenuDialog0xE2_Proc @ 0x00531F60` handles `WM_PAINT` (forwards to child
  `0x71A` for the RA2TS movie), `WM_COMMAND` for button clicks, and otherwise
  delegates to the common dialog proc. **No cursor APIs.**
- `Common_Dialog_Proc @ 0x00622B50` handles WM_DESTROY (2), WM_PAINT (0x0F),
  WM_ERASEBKGND (0x14), WM_DRAWITEM (0x2B), WM_HELP (0x84), WM_INITDIALOG
  (0x110), CTLCOLOR* (0x132-0x138), 0x497, 0x4EC. **No WM_SETCURSOR (0x20),
  no WM_MOUSEMOVE (0x200), no LoadCursorA, no SetCursor, no ShowCursor.**
- `OwnerDraw_Button_00612B70` handles `WM_PAINT` (button bitmap composite),
  `WM_LBUTTONDOWN` (0x201) / `WM_RBUTTONDOWN` (0x203) (play SFX),
  `WM_TIMER` (0x113) (toggle `+0xC5` blink flag), and custom message `0x4DC`
  (track mouse-enter/leave for the blink timer). **No WM_SETCURSOR handling.
  No LoadCursor / SetCursor anywhere in this proc.**

Hover effect is visual-only: the `+0xC5` byte on the per-button state struct
toggles which PCX/SHP frame is composited during the next `WM_PAINT`. The
cursor sprite over the button remains MOUSE.SHA frame 0.

## Key Functions and Addresses

| Address      | Function                                | Role |
|-------------:|------------------------------------------|------|
| 0x00777C30   | CreateMainWindow                         | Registers WNDCLASS with `LoadCursorA(hInst, 0x68)`. Not the visible cursor. |
| 0x007B8730   | WWMouseClass::Constructor                | Calls `ShowCursor(0)` - hides OS cursor for process lifetime |
| 0x005BDF30   | MouseClass::One_Time (vtable[5])         | Loads `MOUSE.SHA` via CDFileClass into `0x00ABF294` |
| 0x005BDA80   | MouseClass::SetCursor (vtable[0x48])     | Selects cursor ID; called with `(0, 0)` at boot |
| 0x005BDC80   | MouseClass::SetMouseShape (vtable[0x4C]) | Reads cursor data table @ 0x0082D028, forwards to WWMouseClass |
| 0x007B8A00   | WWMouseClass::vtable[1]                  | Cursor set-position-and-blit (uses surface pointer) |
| 0x004F4780   | Back-buffer flush                        | Calls WWMouseClass Hide_Mouse / Show_Mouse around blit |
| 0x00531CC0   | MainMenu shell entry (dialog 0xE2)       | Creates dialog; no cursor calls |
| 0x00531F60   | MainMenuDialog0xE2_Proc                  | No cursor calls |
| 0x00622B50   | Common_Dialog_Proc                       | No WM_SETCURSOR handler, no cursor APIs |
| 0x00612B70   | OwnerDraw_Button_00612B70                | Hover is visual (+0xC5 blink); no cursor change |

## Key Data Addresses

| Address      | Content                                  |
|-------------:|------------------------------------------|
| 0x0082604C   | string `"MOUSE.SHA"`                     |
| 0x00ABF294   | global ptr to loaded MOUSE.SHA SHP data  |
| 0x0082D028   | CursorData table (28 bytes/entry)        |
| 0x00B73548   | WNDCLASS hCursor (Win32 resource 0x68; hidden) |
| g_DisplayChain | ptr to active WWMouseClass instance    |

## TS-legacy filtering

- `WWMouseClass` and `MouseClass` are inherited from the TS/WW codebase, but
  both are live in YR: WWMouseClass is unconditionally constructed in
  `WinMain`, and `MouseClass::One_Time` runs every boot via the asset preload
  pass. No conditional gating on SpecialFlags or any TS-only path.
- `AttackCursorOnDisguise`, `AttackCursorOnFriendlies`, `MigAttackCursor`,
  `SabotageCursor`, `CursorCheat` are all **in-game tactical cursor** rules,
  not shell cursor rules. They live in the same MouseClass cursor-table system
  but are only consulted by tactical input paths, never by the shell dialog
  pump. Out of scope per task constraints (flagged in Open Questions).

## Open Questions

1. **Win32 resource 0x68 bitmap content.** `CreateMainWindow` loads cursor
   resource `0x68` and assigns it to the WNDCLASS / SetCursor. Since the OS
   cursor is hidden immediately, this never paints, but the PE resource tree
   was not extracted in this session, so the bitmap behind 0x68 is unknown.
   Likely a standard arrow, but unconfirmed.
2. **Tactical (in-game) cursor surface.** OUT OF SCOPE per task constraints.
   The same MOUSE.SHA + cursor-data-table system drives tactical-view
   attack/move/build/sell/etc cursors with different frame ranges per cursor
   ID. See `MouseClass_research.md` for full coverage.
3. **Custom message `0x4DC` semantics.** Used by `OwnerDraw_Button_00612B70`
   to receive mouse-enter/leave events from a higher subclass; the dispatcher
   wasn't traced. Does not affect cursor (no cursor API called regardless of
   `0x4DC` payload), so noted but not load-bearing.

## Inputs Verified This Session

- String search: no `MOUSE.SHP`, `mouse.shp`, `.cur` strings in gamemd.exe.
  `MOUSE.SHA` confirmed at `0x0082604C`.
- Imports: `LoadCursorA` (EXT:cd), `SetCursor` (EXT:ce), `ShowCursor` (EXT:b5)
  present. `ShowCursor(0)` confirmed at end of `WWMouseClass::Constructor`.
- Decompiled `FUN_00531CC0` (main-menu shell entry): no cursor calls.
- Decompiled `FUN_00622B50` (common dialog proc): no cursor calls, no
  WM_SETCURSOR case.
- Decompiled `OwnerDraw_Button_00612B70`: no cursor calls.
- Decompiled `CreateMainWindow`: confirmed `LoadCursorA(_, 0x68)` and
  `SetCursor` calls.
- Decompiled `WWMouseClass::Constructor`: confirmed `ShowCursor(0)`.
- Decompiled `MouseClass::One_Time` and `SetMouseShape`: confirmed
  `MOUSE.SHA` load and table-driven cursor selection.
- Decompiled `FUN_004F4780` (surface flip): confirmed Hide_Mouse / Show_Mouse
  bracketing around blit.

## Cross-References

- `MouseClass_research.md` - tactical cursor system, full cursor data table
  decoding, animation logic.
- `MAIN_MENU_DIALOG_0XE2_FULL_VISIBLE_COMPOSITION_GHIDRA_REPORT.md` -
  dialog `0xE2` paint and control composition (no cursor coverage there).
- `MAIN_MENU_DIALOG_0XE2_OWNERDRAW_VISUAL_FOLLOWUP_GHIDRA_REPORT.md` -
  owner-draw button visuals (no cursor coverage there).
