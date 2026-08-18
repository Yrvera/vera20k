# Skirmish Button Text Rects and Pressed Offsets 800x600 Trace

Scenario: standard offline Yuri's Revenge Skirmish setup dialog `0x102` at
800x600. Scoped controls are Start Game `0x617`, Choose Map `0x5AA`, and Back
`0x5C0`, released and pressed label states only.

Status: COMPLETE. Ghidra was used only through existing verified reports; no
Ghidra mutation was performed. No Rust, INI, or in-repo docs were modified.

## Current Rust Status Correction - 2026-05-23

The current-Rust FAIL rows below are superseded by
`skirmish-ui/SKIRMISH_OWNERDRAW_BUTTON_PIXEL_RECHECK_800X600_GHIDRA_REPORT.md`.
Current `button_text_rect` keeps the binary right/bottom edges, pressed labels
move by the verified smaller centered delta, and `push_button_label_draw` uses
the yellow enabled text color. Keep the binary facts in this trace, but do not
use its old current-Rust FAIL verdicts as live implementation status.

## Verdict Tally

PASS: 5 | FAIL: 4 | UNCHECKED: 1 | NOT-IMPLEMENTED: 0

PASS means this trace computed literal Rust-side numbers from current source and
matched them to active standard-YR gamemd evidence. Anything without both sides
computed is UNCHECKED.

## Pipeline

Input/state -> `compute_layout(800,600)` button rects ->
`build_shell_text_draws` button label dispatch -> `button_text_rect` ->
`shell_text::draw_in_rect` alignment and scissor -> GPU text sprites.

gamemd path: standard offline dialog `0x102` -> common shell child hook
`FUN_0060F9A0` -> `OwnerDraw_Button_00612B70` -> `FUN_00621040` ->
BitFont draw core.

## Stage Results

| Stage | Our output | gamemd output | Verdict |
|---|---:|---:|---|
| Start Game control rect | `(635,242,162,37)` | `(635,242,162,37)` | PASS |
| Choose Map control rect | `(635,286,162,37)` | `(635,286,162,37)` | PASS |
| Back control rect | `(644,535,156,42)` | `(644,535,156,42)` | PASS |
| Released text rect edges | Start `(635,243,795,280)`, Choose `(635,287,795,324)`, Back `(644,536,798,578)` | Start `(635,243,795,279)`, Choose `(635,287,795,323)`, Back `(644,536,798,577)` | FAIL |
| Pressed text rect edges | Start `(637,247,797,284)`, Choose `(637,291,797,328)`, Back `(646,540,800,582)` | Start `(637,247,795,279)`, Choose `(637,291,795,323)`, Back `(646,540,798,577)` | FAIL |
| Alignment behavior | `ShellAlign::H_CENTER | ShellAlign::V_CENTER`; behavioral bits `0x01|0x04` | `FUN_00621040` flags `0x05`, h-center plus v-center | PASS |
| Pressed effective centered glyph delta | `+2 px` right and `+4 px` down versus released, because width/height remain unchanged | `+1 px` right and `+2 px` down, because right/bottom stay fixed while left/top move | FAIL |
| Enabled button text color source | `SHELL_BUTTON_TEXT_RGB_00000C05 = [0,12,5] / 255` | `DAT_00AC18A4 = 0x0000FFFF`, yellow source color before DirectDraw conversion | FAIL |
| Text draw order relative to art | labels are queued after button PCX instances | text draw after art path completes | PASS |
| Exact localized label bytes | Rust requests `GUI:StartGame`, `GUI:ChooseMap`, `GUI:Back` with English fallbacks | retail text comes through owner state text pointer/string resources | UNCHECKED |

## Computed Rect Details

Rust `button_text_rect(rect, released)` currently computes:

- `x = rect.x`
- `y = rect.y + 1`
- `w = rect.w - 2`
- `h = rect.h`

For Start `(635,242,162,37)`, that is `(x=635,y=243,w=160,h=37)`,
edge rect `(635,243,795,280)`.

gamemd released caller rect is:

- `left = button_left`
- `top = button_top + 1`
- `right = button_left + width - 2`
- `bottom = button_top + height`

For Start `(635,242,162,37)`, that is edge rect `(635,243,795,279)`.
The correct Rust width/height representation would be `(x=635,y=243,w=160,h=36)`.

Rust `button_text_rect(rect, pressed)` currently computes:

- `x = rect.x + 2`
- `y = rect.y + 5`
- `w = rect.w - 2`
- `h = rect.h`

For Start, that is `(x=637,y=247,w=160,h=37)`, edge rect
`(637,247,797,284)`.

gamemd pressed caller rect keeps right and bottom from the released edge rect:
`(637,247,795,279)`. The correct Rust width/height representation would be
`(x=637,y=247,w=158,h=32)`.

## Player-Visible Findings

1. FAIL - Pressed labels move too far. Rust's centered glyph origin moves by
   `+2,+4` from released to pressed; gamemd moves by `+1,+2`. Players see the
   text drop lower and farther right than retail while holding Start, Choose
   Map, or Back.

2. FAIL - Pressed text clipping/scissor is too large. Rust shifts the left/top
   but does not keep the original right/bottom fixed, so the pressed Start rect
   ends at `(797,284)` instead of `(795,279)`. The same error affects Choose and
   Back.

3. FAIL - Released text scissor is one pixel too tall. Rust uses height `37` for
   Start/Choose released labels where gamemd's edge rect implies `36`; Back uses
   `42` where gamemd implies `41`.

4. FAIL - Enabled button label color source is wrong. Current Rust uses a
   `0x00000C05`-style RGB constant, while the verified button path uses
   `DAT_00AC18A4 = 0x0000FFFF` before DirectDraw display-format conversion. This
   is a direct explanation for dark/bad-looking Skirmish button text.

## Adjacent Findings

- The existing test `button_text_rect_follows_owner_draw_caller_contract` asserts
  the current Rust width/height values, but those values are not the gamemd
  edge-rect contract. It should assert fixed right/bottom semantics instead.
- `SKIRMISH_OWNERDRAW_BUTTON_PIXEL_LAYOUT_GHIDRA_REPORT.md` contains older prose
  that says flags `0x0C`; the newer text-renderer contract resolves the live
  wrapper flag as `0x05`. Both imply the same intended behavior here: horizontal
  center plus vertical center.
- Disabled button text color and alpha are outside this slot's released/pressed
  scope.

## Sources

- Rust read-only scan: `src/ui/skirmish_shell/layout.rs`,
  `src/app_skirmish_shell_render.rs`, `src/render/shell_text.rs`.
- Verified research docs:
  `skirmish-ui/SKIRMISH_OWNERDRAW_BUTTON_PIXEL_LAYOUT_GHIDRA_REPORT.md`,
  `skirmish-ui/SKIRMISH_TEXT_RENDERER_CALLER_PIXEL_CONTRACT_GHIDRA_REPORT.md`,
  `SHELL_BUTTON_PAINT_DETAILS_GHIDRA_REPORT.md`,
  `traces/SKIRMISH_OWNER_DRAW_CONTROLS_VISUAL_STATES_TRACE.md`.
- gamemd evidence named by those docs: standard dialog `0x102`,
  `FUN_0060F9A0`, `OwnerDraw_Button_00612B70`, `FUN_00621040`, and
  `DAT_00AC18A4`.
