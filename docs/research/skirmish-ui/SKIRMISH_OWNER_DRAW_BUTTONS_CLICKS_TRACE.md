# Skirmish Owner-Draw Buttons Click Trace

Date: 2026-05-20

Scenario: At 800x600 in Skirmish setup dialog `0x102`, press and release Start Game, Choose Map, and Back. Verify hit rects, control IDs `0x617` / `0x5AA` / `0x5C0`, pressed PCX skin selection, label pressed offset, and release-match behavior.

Scope is limited to these three owner-draw buttons and this click scenario. Ghidra MCP use was read-only: `OwnerDraw_Button_00612B70`, `FUN_006ae3f0`, and `FUN_006acee0` were decompiled only.

## Evidence Base

- `SKIRMISH_SHELL_LAYOUT_ASSETS_GHIDRA_REPORT.md`: dialog resource `0x102`, button control IDs, DLU resource rects, and owner-draw styles.
- `SKIRMISH_SHELL_VIEWPORT_ORIGIN_FOLLOWUP_GHIDRA_REPORT.md`: final 800x600 button rects after shell right-anchor/back-button helpers.
- `SKIRMISH_OWNERDRAW_CALLBACKS_GHIDRA_REPORT.md` and follow-up: callback assignment and 30px PCX family selection.
- `SKIRMISH_SHELL_ACTIVE_RENDER_PATH_LIVE_GHIDRA_REPORT.md`: active YR confirmation for `0x00612B70` on Start/Choose/Back.
- `SHELL_PCX_BUTTON_TILE_AND_CAP_GEOMETRY_GHIDRA_REPORT.md`: PCX branch geometry and pressed-state art/text offsets. Its main-menu active-YR header was later corrected, but the PCX branch geometry remains applicable to active offline Skirmish `0x102` per the Skirmish live report.
- Fresh read-only Ghidra spot-check: `0x00612B70` handles `WM_LBUTTONDOWN`, calls previous WndProc for default button processing, and uses `piVar17[0x3a] & 1` as the pressed bit during paint. `FUN_006ae3f0` routes `WM_COMMAND` to `FUN_006acee0`; `FUN_006acee0` handles `0x5AA`, `0x5C0`, and `0x617`, with Start/Back gated on notification `0`.

## Pipeline

Pointer down in one of three final rects -> Rust stores `pressed_owner_draw_button` -> Rust paints the matching button with down skin and label offset -> pointer up hit-tests again -> Rust fires only if pressed identity equals released identity -> action is mapped to StartGame, ChooseMap, or BackOrExit.

In gamemd, the active path is Win32 dialog `0x102`: the child button WndProc is subclassed to `OwnerDraw_Button_00612B70`; the callback plays shell click sound on `WM_LBUTTONDOWN`, delegates unhandled/default mouse processing to the previous Button WndProc, paints from the active Button pressed bit, and the dialog proc consumes the resulting `WM_COMMAND`.

## Stage Results

| Stage | Rust output | gamemd output | Verdict |
|---|---|---|---|
| Active routing | Buttons modeled as `OwnerDrawButton::{StartGame0x617, ChooseMap0x5aa, Back0x5c0}` in `src/ui/skirmish_shell/state.rs:9` | Dialog resource `0x102` buttons `0x617/0x5AA/0x5C0` route to `0x00612B70`; active in standard YR | PASS |
| 800x600 final hit rects | Start `(635,242,162,37)`, Choose `(635,286,162,37)`, Back `(644,535,156,42)` from `src/ui/skirmish_shell/layout.rs:206` | Same final rects in viewport follow-up: right-anchor default inset for Start/Choose; `SDBTNANM.SHP` Back helper | PASS |
| Hit edge rule | `RectPx::contains` uses `x >= left`, `y >= top`, `x < right`, `y < bottom` at `src/ui/skirmish_shell/layout.rs:23` | Win32 child window client hit-testing is half-open in pixel coordinates; no contrary gamemd override found | PASS |
| Control identity on press | Press inside Start/Choose/Back returns exact enum identity at `src/ui/skirmish_shell/state.rs:105` | `FUN_006acee0` command cases include `0x5AA`, `0x5C0`, and `0x617`; `0x00612B70` is assigned to those controls | PASS |
| PCX family / pressed skin | Up `bue_li30/mi30/ri30`, down `bde_li30/mi30/ri30` in `src/app_skirmish_shell_render.rs:187` and pressed comparisons at `:566`, `:574`, `:582` | Active Skirmish path selects `30` family for 37px controls and state char `'u'` or `'d'` | PASS |
| Pressed art vertical placement | Rust draws each PCX piece at `rect.y` and sizes it to `rect.h` (`37` for Start/Choose, `42` for Back) at `src/app_skirmish_shell_render.rs:285` | gamemd uses 30px PCX art height for the 37px normal buttons: unpressed `art_y = screen_y + 3`, pressed `art_y = screen_y + 5` | FAIL |
| Cap/middle geometry | Rust creates non-overlapping segments: middle ends at `rect.x + rect.w - right_w`, right starts there at `src/app_skirmish_shell_render.rs:215` and `:228` | gamemd middle dest width is `button_width - 10`, starts at `x+7`, and overlaps the right cap by 7px; for width 162, middle dest width is 152 and visible middle is 145 | FAIL |
| Pressed label offset | Rust only adds `y_offset = 2`; `TextRect.x` remains `rect.x` at `src/app_skirmish_shell_render.rs:633` and `:680` | gamemd pressed text rect shifts left by +2 and top by +4, producing net glyph delta `+1px right, +2px down` after centering | FAIL |
| Release-match behavior | Rust takes the stored press identity and fires only when it equals the release hit identity at `src/app.rs:570` | gamemd delegates mouse default processing to the previous Button WndProc and consumes the resulting `BN_CLICKED`/notification `0`; exact mismatched-release negative case was not independently simulated | UNCHECKED |

## Failures

1. Pressing any of the three buttons draws the PCX art too tall and too high. The player sees the down skin fill the full control rect instead of a 30px strip centered within the button and shifted down by 2px while held.

2. The cap/middle/right composition does not match gamemd's overlap. Rust crops the middle segment to avoid the right cap, while gamemd draws a wider middle rect and lets the right cap overwrite the final 7px. The player-visible seam and texture phase can differ on all three buttons.

3. Pressed labels move down but not right. In gamemd, held button text moves by net `(+1,+2)` pixels; in Rust it moves `(0,+2)`. The missing 1px right shift is small but visible because it is exactly the tactile click cue.

## Not Implemented

None for the requested surfaces. The three requested controls exist, have identities, render up/down PCX skins, and have release-gated actions in Rust.

## Adjacent Findings

- Choose Map's actual post-click action differs beyond this trace's requested fields: gamemd opens/runs the map-selection flow, while Rust currently cycles the selected map in `apply_action`. This belongs to the separate Choose Map action trace.
- Start Game and Back command side effects were not audited here beyond click activation routing.
- Gamemd disabled-state alpha and `bud_*` non-use are documented but not part of this enabled-button click scenario.

## Verdict Tally

PASS: 5 | FAIL: 3 | UNCHECKED: 1 | NOT-IMPLEMENTED: 0

## Status

COMPLETE
