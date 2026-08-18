# Main Menu 0xE2 Status and Button Composition Design

**Date:** 2026-07-18  
**Status:** Approved  
**Scope:** Initial main-menu dialog (`0xE2`) hover/status behavior, Exit geometry,
and owner-draw button micro-composition

## Goal

Close the verified player-visible disparities in the initial Yuri's Revenge main
menu without broadening the change into other pregame dialogs or altering the
in-game tooltip mechanism.

The result must preserve the existing correct title position, the five primary
button rectangles, native SHP frame selection, and shell render order while
matching the active `gamemd.exe` status-line and pressed-button mechanisms.

## Evidence Corrections

Live decompilation and assembly checks corrected several stale conclusions in the
existing research corpus:

- The main-menu status static (`0x695`) is updated immediately by the dialog's
  `WM_NCHITTEST` path. It is not the delayed popup-tooltip mechanism.
- Exit (`0x3EE`) does enter the owner-draw button reposition path through the
  dialog/control predicate and is resized to `156x42`. Its native Y position is
  the top of the last right-panel tile, not the raw resource top.
- The title (`0x694`) is already correctly placed at X=635 in the 800x600
  fixture after the right-anchor and nudge helpers are composed.
- A pressed type-1 owner-draw button selects SDBTNANM frame 4 without translating
  the SHP art.
- The pressed label changes its full clipping rectangle. Describing it only as a
  one-pixel X translation is incomplete.

These corrections must be reflected in the relevant research reports alongside
the code change so later parity work does not reuse the stale claims.

## Verified Native Contract

### Hover status

- Each mouse movement over dialog `0xE2` resolves the child under the pointer.
- A recognized main-menu button maps to its `STT:MainButton*` CSF string.
- The string is sent immediately to status static `0x695`.
- Moving over a non-button clears the status string.
- Without further pointer movement, the last resolved string remains displayed.
- There is no one-second delay, ten-second expiry, popup background, or
  `[Options] ToolTips` gate in this path.
- The text uses GAME.FNT, yellow (`#FFFF00`), left horizontal alignment, and
  vertical centering.
- The status remains in the shell text layer. At 800x600 its rectangle is
  `(10, 579, 455, 20)`; at 1024x768 it is `(122, 663, 455, 20)`.

### Button geometry

- The five non-Exit owner-draw buttons remain `156x42` at X=644 with Y positions
  `199, 241, 283, 325, 367` in the 800x600 fixture.
- Exit is `156x42`, right-aligned to the panel, and placed at
  `right_panel.bottom.y - 42`.
- Exact Exit fixtures are:

| Resolution | Rectangle |
|---|---|
| 640x480 | `(484, 409, 156, 42)` |
| 800x600 | `(644, 535, 156, 42)` |
| 1024x768 | `(756, 619, 156, 42)` |

### Owner-draw composition

- SDBTNANM frames remain 2 for default, 3 for highlighted, and 4 for pressed.
- Pressing changes the selected frame but does not translate the SHP art.
- Label rectangles, expressed as `(x, y, width, height)`, are:
  - normal: `(x, y + 1, width - 2, height - 1)`
  - pressed: `(x + 2, y + 5, width - 4, height - 5)`
- With horizontal and vertical centering, the glyph result moves approximately
  one pixel right and two pixels down, but the exact rectangle and clipping
  boundaries are the parity contract.

## Chosen Architecture

Use the existing main-menu interaction state as the native-shaped authority for
the status line. Do not route main-menu status through the shared delayed
`TooltipService`.

Data flow:

`pointer move -> DialogController hit test -> hovered control in MainMenuShellState -> CSF key lookup -> status static 0x695`

This preserves one authoritative hit-test path for both interaction and display.
The resolved shell layout remains shared by rendering and pointer hit testing, so
the Exit correction applies consistently to both.

## Component Changes

### Main-menu state and rendering

- Read `hovered_owner_draw_button` when building the main-menu text layer.
- Map known controls through the existing control-to-tooltip-CSF-key helper.
- Resolve the mapped CSF text with the existing shell localization path.
- Render no status label when no recognized button is hovered.
- Use vertical-centering alignment only; omission of horizontal-centering retains
  native left alignment.
- Leave the title's X position and the five primary button rectangles unchanged.

### Tooltip ownership

- Remove main-menu region registration and namespace filtering from
  `app_tooltips` and the main-menu renderer.
- Keep `TooltipService` behavior unchanged for the in-game/sidebar UI, including
  its existing delay and expiry semantics.
- Do not introduce a new cross-dialog status service in this slice.

### Exit layout

- Replace the misleading raw-top Exit anchor with a bottom-row owner-draw anchor.
- Compute Y from the resolved right-panel bottom rectangle and the native tile
  height.
- Keep the native SHP-derived width and height and existing right alignment.

### Pressed composition

- Set the main-menu button policy's art sink to zero.
- Reuse one exact owner-draw label-rectangle helper for the initial menu and the
  quit-confirm modal, whose current rectangle calculation already matches the
  verified formula.
- Rename the helper to describe owner-draw behavior rather than modal ownership.
- Do not broaden this helper into Skirmish or other dialogs without their own
  evidence pass.

## Tiny-Detail Ledger

| Detail | Required outcome |
|---|---|
| Status timing | Immediate on pointer movement |
| Status persistence | Remains while pointer is stationary |
| Background movement | Clears status |
| Status font/color | GAME.FNT / `#FFFF00` |
| Status alignment | Left plus vertical center |
| Tooltip option gate | None for main-menu status |
| Status render layer | Shell text layer, before cursor |
| Title position | Preserve X=635 at 800x600 |
| Primary buttons | Preserve current five rectangles |
| Exit size | `156x42` |
| Exit Y | Bottom-panel Y minus 42 |
| Art press motion | None |
| Pressed frame | SDBTNANM frame 4 |
| Label press result | Exact native rectangle, not an approximate offset |
| Website control | No added visible seventh button |
| Simulation/RNG | No change |

## Failure and Fallback Behavior

- An unrecognized hovered control produces no status label rather than displaying
  stale text from a different button.
- CSF resolution continues to use the existing localization fallback behavior;
  this design adds no new error channel or hardcoded English strings.
- Missing render assets retain the renderer's existing handling. This work does
  not add alternate art or guessed constants.

## Verification

Add or update focused tests that prove:

1. Exit resolves to the exact 640x480, 800x600, and 1024x768 fixtures.
2. The five primary 800x600 button rectangles and title position do not change.
3. Rendering and hit testing consume the same corrected Exit rectangle.
4. Known hovered controls map to their exact `STT:MainButton*` keys; no control
   maps to no status text.
5. The status label uses left plus vertical-center alignment.
6. Normal and pressed label rectangles match the exact boundary formulas.
7. Pressed and unpressed button art use the same destination origin while frame
   selection still changes to frame 4.
8. Existing delayed in-game tooltip tests continue to pass unchanged.

Run focused tests serially, then one final `cargo check -q`, after checking for an
active Cargo owner. Format only edited Rust files and inspect the diff for
unrelated workspace changes.

## Non-Goals

- Skirmish, single-player, multiplayer, or other pregame dialog status behavior.
- New generic shell-status infrastructure.
- Changes to title wording, version text, movie playback, shell audio, cursor
  behavior, simulation state, save data, RNG, or dependencies.
- A broad owner-draw refactor or asset rebaseline.

## Acceptance Criteria

- Main-menu hover help behaves as the immediate persistent native status line.
- Exit occupies the exact native rectangle at all three fixture resolutions.
- Pressed buttons change frame and label composition without moving their art.
- Correct existing main-menu geometry and title placement are preserved.
- In-game tooltip behavior is unchanged.
- Research reports no longer state the disproven Exit and pressed-composition
  claims.
