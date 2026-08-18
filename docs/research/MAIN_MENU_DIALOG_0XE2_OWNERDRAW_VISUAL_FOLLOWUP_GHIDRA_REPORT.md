# Main Menu Dialog 0xE2 Owner-Draw Visual Follow-up - Ghidra Report

Date: 2026-05-17

Scope: targeted re-investigation of the standard Yuri's Revenge initial main menu
dialog `0xE2`, focused on the user's suspicion that the current Rust shell looks
wrong despite using apparently correct button assets. This pass extends:

- `MAIN_MENU_SIDEBAR_GHIDRA_REPORT.md`
- `MAIN_MENU_VISUAL_ASSETS_GHIDRA_REPORT.md`
- `SKIRMISH_OWNERDRAW_CALLBACKS_FOLLOWUP_GHIDRA_REPORT.md`
- in-repo `docs/gap-scans/2026-05-17-disparity-scan-main-menu-shell-assets.md`

No Rust code was changed by this investigation.

## Executive Summary

The asset family is still correct: standard YR dialog `0xE2` uses the RA2TS Bink
panel plus generic shell owner-draw PCX buttons generated as
`bue_li30/mi30/ri30.pcx` and `bde_li30/mi30/ri30.pcx`. This is not the TS
`GraphicMenu` / `Title.PCX` path and not the Skirmish right-panel SHP stack.

> **CORRECTION (2026-05-30):** The `bue/bde` PCX claim describes the **type-0 default**
> branch of `OwnerDraw_Button_00612B70` and is **NOT the active art** for dialog `0xE2`.
> The five non-Exit `0xE2` buttons are resized to the `SDBTNANM.SHP` cell (156×42) by
> `FUN_0060B000` and painted with SDBTNANM frames 2/3/4 (the type-1 branch); `bue/bde` are
> preloaded but unused on this path. The Exit button `0x3EE` is special-cased (not resized,
> raw ≈162×37 @ x=638). §3/§4 below correctly describe the **inactive** PCX branch, not
> what `0xE2` actually paints. Evidence: the button window equals the SDBTNANM frame rect
> (x=644 = `RightPanel__ComputeLayoutRects` SDBTNANM rect); `decompile_function 0x00608CD0`
> / `0x0060B000` / `0x00612B70` / `0x0060F9A0`. Full analysis:
> `MAIN_MENU_0XE2_BUTTON_PAINT_AND_REPOSITION_FORK_GHIDRA_REPORT.md`.

The deeper pass found two strong visual parity issues in the current Rust main
menu implementation:

1. **Button text color is wrong.** `OwnerDraw_Button_00612B70` passes the common
   default color `DAT_00AC18A4 = 0x0000FFFF` to `FUN_00621040`. That wrapper
   interprets the low, middle, and high bytes as RGB, so normal enabled shell
   button text is yellow `#FFFF00`. Current Rust uses a dark green-ish constant
   named `SHELL_BUTTON_TEXT_RGB_00000C05`.
2. **Button PCX height should not be stretched.** The binary selects the `30`
   family because the dialog button client height is 37 px, then draws the 30 px
   cap/middle/right strip vertically centered inside the 37 px client rect. The
   middle piece is tiled. Current Rust draws the correct pieces but stretches
   them to the whole scaled button rect height.

`0x71C` is still not proven to be a missing visible asset. It is a Static
control, not a Button; common owner-draw dispatch maps it to
`OwnerDraw_Static_006153E0`. No main-menu creation/refresh code found in this
pass sends it an image, movie, bitmap, or tooltip-related custom message.

## Verified Findings

### 1. Standard initial menu still creates dialog `0xE2`

Evidence:

- `FUN_00531CC0`
- `FUN_0052B9B0`

`FUN_00531CC0` sets the loop result to `0x12`, creates RT_DIALOG `0xE2`, stores
the loop-result pointer at window long offset `8`, centers/shows the dialog, and
then only directly manipulates child `0x71A`.

For child `0x71A`:

```text
x = 0 if screen_width  < 801 else (screen_width  - 800) / 2
y = 0 if screen_height < 601 else (screen_height - 600) / 2
SetWindowPos(0x71A, x, y, -1, -1, 0xD)
SendMessage(0x71A, 0x4E3, 1, 0)
SendMessage(0x71A, 0x4E4, 0, screen_width == 640 ? "Ra2ts_s" : "Ra2ts_l")
```

`FUN_0052B9B0` repeats the same sequence. No equivalent setup for `0x71C` was
found in these functions.

Confidence: High.

Active in YR: Yes.

### 2. `0x71C` is a Static owner-draw control, not a shell button

Evidence:

- RT_DIALOG `0xE2` from prior report: `0x71C`, class Static, rect
  `447,29,61,33`, style `0x50000007`, no title.
- `FUN_0060F9A0`
- `OwnerDraw_Static_006153E0`

`FUN_0060F9A0` dispatches by Win32 class before button-style subdispatch:

```text
class "Static" -> OwnerDraw_Static_006153E0, kind 0x2
class "Button" with style bits 0x0B -> OwnerDraw_Button_00612B70
```

Therefore `0x71C` cannot use the button PCX path even though its static style
low bits are `7`. It is caught by the Static class branch first.

On custom `0x497`, `OwnerDraw_Static_006153E0` initializes a normal static state
with type/kind `0`, default text color slot, and text pointer from the dialog
title if present. The dialog template title for `0x71C` is empty. The main menu
creation and refresh functions only send movie messages to `0x71A`, not to
`0x71C`.

Interpretation: `0x71C` should remain documented as an open visual question, but
this pass did **not** find binary evidence for a missing website icon/image. The
tooltip table mentions control `0x55F` for `STT:MainButtonYuriWebSite`; that is
a different control id from `0x71C`.

Confidence: High for subclass identity; Medium for "not visible", because a
broader whole-shell xref scan to every possible `SendMessage(..., 0x71C, ...)`
was not completed.

Active in YR: Yes for the subclass path; visible output is unproven.

### 3. Button asset names are correct and not TS legacy

Evidence:

- `OwnerDraw_Button_00612B70`
- format strings:
  - `0x0083589C`: `b%c%c_li%d.pcx`
  - `0x0083588C`: `b%c%c_mi%d.pcx`
  - `0x0083587C`: `b%c%c_ri%d.pcx`

For normal enabled shell buttons:

- first `%c` is `'u'` for up/unpressed and `'d'` for down/pressed;
- second `%c` is hardcoded `'e'`;
- disabled style forces the up art and applies alpha `0x80`, rather than using
  `bud_*` on this path;
- button height family is selected from `24` and `30` thresholds.

For dialog `0xE2`, the resource DLU size `108x23` maps to roughly `162x37` px
under the shell font. Since 37 is at least 30, the selected family is:

| State | Left | Middle | Right |
|---|---|---|---|
| Up | `bue_li30.pcx` | `bue_mi30.pcx` | `bue_ri30.pcx` |
| Down | `bde_li30.pcx` | `bde_mi30.pcx` | `bde_ri30.pcx` |

Confidence: High.

Active in YR: Yes.

### 4. Button PCX pieces are drawn at selected art height, not stretched

Evidence:

- `OwnerDraw_Button_00612B70`
- `FUN_006BA3E0`

The button callback chooses a size suffix, stores it as the selected art height,
and computes a vertical placement:

```text
selected_height = 24 or 30
button_art_y = client_top + (client_height - selected_height) / 2
if pressed: button_art_y += 2
```

For the normal `0xE2` 37 px button client:

```text
up art y offset      = (37 - 30) / 2 = 3
pressed art y offset = 3 + 2 = 5
```

The left and right caps are drawn through the surface blit path. The middle
piece is passed to `FUN_006BA3E0`, which tiles source pixels into the destination
rectangle; it uses modulo addressing over the source surface dimensions, so this
is tiling, not scaling.

Important tiny detail: the `30` suffix is not "stretch to 37 px". It is "choose
the 30 px art family because the control is at least 30 px high, then center that
30 px strip in the client."

Confidence: High.

Active in YR: Yes.

### 5. Normal shell button text color is yellow `#FFFF00`

Evidence:

- `FUN_0060F9A0`
- `OwnerDraw_Button_00612B70`
- `FUN_00621040`

During common owner-draw setup, `FUN_0060F9A0` initializes:

```text
DAT_00AC18A4 = 0x0000FFFF
```

In the normal PCX-button paint path, `OwnerDraw_Button_00612B70` loads that
global into the color variable before drawing. Later, if the control has text
and is a normal PCX button, it calls:

```text
FUN_00621040(rect, text, color, 5, 0x0C, 0, 0, 0)
```

`FUN_00621040` splits the color as:

```text
red   = color & 0xFF
green = (color >> 8) & 0xFF
blue  = (color >> 16) & 0xFF
```

Therefore `0x0000FFFF` means:

```text
R = 255
G = 255
B = 0
```

Normal enabled shell button text is yellow `#FFFF00`. This is independent of the
PCX embedded palette because GAME.FNT text is rasterized through the font draw
wrapper and the active DirectDraw pixel format conversion.

Current Rust observation:

- `src/app_main_menu_shell_render.rs` defines
  `SHELL_BUTTON_TEXT_RGB_00000C05 = [0.0, 12.0 / 255.0, 5.0 / 255.0]`.

That color does not match the binary's normal enabled button text color.

Confidence: High.

Active in YR: Yes.

### 6. Owner-draw PCX palette conclusion remains correct

Evidence:

- `SKIRMISH_OWNERDRAW_CALLBACKS_FOLLOWUP_GHIDRA_REPORT.md`
- `OwnerDraw_Button_00612B70`
- owner-draw PCX loader path `0x006B9D00..0x006BA09F`

The PCX button pieces are decoded through their embedded 256-color PCX VGA
palette and converted to the active 16-bit surface format. This pass found no
new evidence that `SHELL.PAL`, `SHELL2.PAL`, `SDBTNANM.PAL`, `DIALOG.PAL`, or
`MAINBTTN.PAL` are used for the PCX button surfaces.

Conclusion: the likely visual mismatch is not "wrong PCX palette" for the button
art. It is the text color and the art stretch/scale behavior.

Confidence: High.

Active in YR: Yes.

### 7. Mouse-down sound binding remains correct

Evidence:

- `OwnerDraw_Button_00612B70`
- prior sound reports mapping `RulesClass + 0x188` to `GUIMainButtonSound`

The callback handles `WM_LBUTTONDOWN` (`0x201`) and `WM_LBUTTONDBLCLK`
(`0x203`) by playing the sound at `RulesClass + 0x188` when the control is not
disabled. This confirms the main-menu click sound belongs on mouse-down, not on
successful release/activation.

There is also an internal paint/state-transition sound site at `0x00613289`
inside `OwnerDraw_Button_00612B70` for a button moving from `'u'` to `'d'`. It
loads `RulesClass + 0x70C` = `GenericClick` (default INI value `MenuClick`),
**not** `+0x750` = `ShellButtonSlideSound`. Do not treat the paint-transition
site as a separate asset selection path. The player-visible mouse-down trigger
(`+0x188`, `GUIMainButtonSound`) is still the important input-ordering fact for
the Rust implementation. See `SHELL_BUTTON_SLIDE_SOUND_CALL_SITE_GHIDRA_REPORT.md`
for the resolved attribution and the actual `ShellButtonSlideSound` consumer at
`0x00607F59` (slide-in completion cue, Load-Game-success path).

Confidence: High for mouse-down binding; High for the paint-transition site
attribution (verified via slot-2 swarm re-investigation).

Active in YR: Yes.

## Current Rust Drift Notes

Observed current Rust implementation after the responsive-shell change:

| Area | Current Rust | Binary finding | Status |
|---|---|---|---|
| RA2TS panel asset | `ra2ts_s.bik` at width 640, otherwise `ra2ts_l.bik` | Same selection rule | Correct asset rule |
| Overall scaling | `compute_responsive_layout` scales 800x600 logical shell to full swapchain | Retail keeps fixed 800x600 shell geometry and centers movie for larger modes | Intentional user-approved drift |
| Button PCX names | Loads `bue_*30` / `bde_*30` | Correct names for 37 px controls | Correct |
| Button PCX height | Draws segments at `rect.h` | Draws selected 30 px art strip centered in 37 px client | Likely visible bug |
| Button text color | Dark green-ish `[0,12,5]` | Yellow `#FFFF00` | Confirmed visible bug |
| `0x71C` | Layout exists, render path does not draw it | No image/movie setup found; Static owner-draw with no title | Keep as open, not yet a proven bug |

> **CORRECTION (2026-05-30):** The `Button PCX names` / `Button PCX height` rows above are
> superseded — the active `0xE2` button art is `SDBTNANM.SHP` (type 1), not `bue/bde` PCX
> (type 0). The real geometry deltas are window size (gamemd 156×42 vs Rust 162×37), X
> (gamemd 644 flush-right vs Rust 635), grid-snapped Y, and Exit `0x3EE` being
> special-cased. See `MAIN_MENU_0XE2_BUTTON_PAINT_AND_REPOSITION_FORK_GHIDRA_REPORT.md`.

## Practical Next Fix Targets

These are research-derived implementation implications, not code changes:

1. Change initial main-menu owner-draw button text to the binary color
   `#FFFF00` for the normal enabled path.
2. Stop stretching the `30` family button PCX pieces to the full button rect
   height. Draw them at source/art height and vertically center them inside the
   scaled button client. If the responsive shell remains approved, scale the
   source art height by the same responsive Y factor, rather than using the full
   button rect height as the art height.
3. Keep using the embedded PCX palettes for button art.
4. Do not add TS `GraphicMenu`, `Title.PCX`, Skirmish right-panel SHPs, or
   `bud_*` button assets to the initial `0xE2` main menu.
5. Leave `0x71C` alone until a retail screenshot or a broader `SendMessage` /
   dialog-control xref pass proves a visible image or website control is active.

## Sources Checked

Fresh Ghidra functions decompiled/disassembled in this pass:

- `FUN_00531CC0`
- `FUN_0052B9B0`
- `FUN_0060F9A0`
- `OwnerDraw_Button_00612B70`
- `OwnerDraw_Static_006153E0`
- `FUN_00621040`
- `FUN_006211D0`
- `FUN_00622B50`
- `FUN_006040B0`
- `FUN_006BA3E0`
- `FUN_006BA580`

Repo files inspected:

- `src/app_main_menu_shell_render.rs`
- `src/ui/main_menu_shell/layout.rs`
- `docs/gap-scans/2026-05-17-disparity-scan-main-menu-shell-assets.md`

Prior standalone reports referenced:

- `MAIN_MENU_SIDEBAR_GHIDRA_REPORT.md`
- `MAIN_MENU_VISUAL_ASSETS_GHIDRA_REPORT.md`
- `BITFONT_SHELL_TEXT_GHIDRA_REPORT.md`
- `SKIRMISH_OWNERDRAW_CALLBACKS_FOLLOWUP_GHIDRA_REPORT.md`
- `SKIRMISH_SHELL_RIGHT_PANEL_BACKGROUND_PALETTE_FOLLOWUP_GHIDRA_REPORT.md`

## Related reports (added 2026-05-18 main-menu --area swarm)

The 2026-05-18 main-menu swarm produced five new reports. Most relevant
to this doc's owner-draw and sound-binding analysis:

- `SHELL_BUTTON_SLIDE_SOUND_CALL_SITE_GHIDRA_REPORT.md` — **resolves the
  attribution ambiguity in §7 (above).** The paint-transition site at
  `0x00613289` inside `OwnerDraw_Button_00612B70` loads `RulesClass +
  0x70C` (`GenericClick`, default `MenuClick`), not `+0x750`
  (`ShellButtonSlideSound`). The real `ShellButtonSlideSound` consumer is
  at `0x00607F59` in the slide-in animation function, gated on the
  open/show direction (Load Game success path).
- `SDBTNANM_FRAME10_OVERLAY_CONDITION_GHIDRA_REPORT.md` — partial: the
  right-panel frame-10 overlay branch is a binary highlight selector
  gated by WindowExtra record byte `+0xD8`. Sticky-on; clearer dormant.

Other reports in the same swarm (less directly relevant to this doc):

- `MAIN_MENU_MUSIC_TRACK_AND_LOOP_GHIDRA_REPORT.md`
- `EVA_WELCOME_BACK_MAIN_MENU_TRIGGER_GHIDRA_REPORT.md` (verified-negative)
- `QUIT_CONFIRM_DIALOG_MAIN_MENU_GHIDRA_REPORT.md`
