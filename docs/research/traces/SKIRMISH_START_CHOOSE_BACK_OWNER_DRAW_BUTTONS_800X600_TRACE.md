# Skirmish Start/Choose/Back Owner-Draw Buttons 800x600 Trace

Date: 2026-05-22

Scenario: standard offline Yuri's Revenge Skirmish setup dialog `0x102` at `800x600`, limited to released and pressed visual states for Start Game `0x617`, Choose Map `0x5AA`, and Back `0x5C0`.

Status: COMPLETE. Ghidra use was read-only. No Rust, INI, or in-repo docs were modified.

## Current Rust Status Correction - 2026-05-23

## Superseded Asset-Family Correction - 2026-05-24

The PCX asset-family conclusion in this trace is superseded for Start Game
`0x617`, Choose Map `0x5AA`, and Back `0x5C0`. A later classifier recheck found
that the active Skirmish shell setup path sets these right-panel buttons to
owner-draw type `1`, and the type-1 branch of `OwnerDraw_Button_00612B70` draws
`SDBTNANM.SHP` frames `2`/`4`, not the generic gray PCX branch. Use
`skirmish-ui/SKIRMISH_RIGHT_PANEL_BUTTON_SDBTNANM_TYPE1_RECHECK_GHIDRA_REPORT.md`
for the current asset-family contract. Do not use this trace to restore
`bue_*30.pcx` / `bde_*30.pcx` for the standard Skirmish sidebar buttons.

The current-Rust FAIL verdicts for art y, pressed art movement, button rects,
text rects, and enabled text color are superseded by
`skirmish-ui/SKIRMISH_OWNERDRAW_BUTTON_PIXEL_RECHECK_800X600_GHIDRA_REPORT.md`.
Current Rust now matches 800x600 owner-draw button snap rects, native 30 px art
y centering, pressed +2 art movement, fixed-right/bottom text rects, and
yellow enabled label color. The remaining scoped visual mismatch is the middle
PCX source phase: Rust preserves the verified 7 px right-cap overlap, but the
middle PCX source phase still needs `FUN_006BA3E0`'s centered crop offset
before modulo tiling.

## Evidence Base

- Rust read-only scan: `src/ui/skirmish_shell/layout.rs`, `src/ui/skirmish_shell/state.rs`, `src/app_skirmish_shell_render.rs`, `src/app.rs`, `src/render/skirmish_shell_chrome.rs`.
- Verified docs: `skirmish-ui/SKIRMISH_OWNERDRAW_BUTTON_PIXEL_LAYOUT_GHIDRA_REPORT.md`, `skirmish-ui/SKIRMISH_SHELL_VIEWPORT_ORIGIN_FOLLOWUP_GHIDRA_REPORT.md`, `traces/SKIRMISH_OWNER_DRAW_BUTTON_PRESS_RELEASE_TRACE.md`, `skirmish-ui/SKIRMISH_SHELL_BACKGROUND_TEXT_PREVIEW_GHIDRA_REPORT.md`.
- Fresh read-only Ghidra spot-check: `OwnerDraw_Button_00612B70`, `FUN_006BA3E0`, `0x0060B1D0`, and `0x0060B350`.

## Pipeline

Rust path:

`compute_layout(800,600)` -> `hit_test_owner_draw_button` -> `pressed_owner_draw_button` -> `push_button_30` -> `button_text_rect` -> GPU batch sprites/text.

gamemd path:

standard Skirmish dialog `0x102` -> shell child resize/fixup callbacks -> Button controls subclassed to `OwnerDraw_Button_00612B70` -> Win32 Button pressed bit -> PCX cap/middle/right paint -> `FUN_00621040` text draw.

## Stage Results

| Stage | Rust output | gamemd output | Verdict |
|---|---|---|---|
| Active standard-YR route | `OwnerDrawButton::{StartGame0x617, ChooseMap0x5aa, Back0x5c0}` drives render/input | dialog `0x102` controls `0x617/0x5AA/0x5C0` route to `OwnerDraw_Button_00612B70`; active in standard YR | PASS |
| Final 800x600 control rects | Start `(635,242,162,37)`, Choose `(635,286,162,37)`, Back `(644,535,156,42)` | same: Start/Choose via `0x0060B1D0` default right-panel inset; Back via `0x0060B350` and `SDBTNANM.SHP=156x42` | PASS |
| Hit-test rect identity | `hit_test_owner_draw_button` uses those same three layout rects | Win32 child windows are moved to those same rects by the active resize callbacks | PASS |
| Released/pressed asset family | released `bue_li30/mi30/ri30`; pressed `bde_li30/mi30/ri30` | same formatted filenames in `OwnerDraw_Button_00612B70`; suffix `30` selected for 37/42 px clients | PASS |
| Native art height | sprite height is `entry.pixel_size[1]`, not `rect.h`; 30-family art remains 30 px | retail blits/tiles the selected 30-family PCX art at native 30 px height | PASS |
| Art vertical placement | all three buttons draw PCX art at `rect.y`; pressed state does not move art | art y is `top + (height - 30) / 2`, plus `+2` when pressed: Start `245/247`, Choose `289/291`, Back `541/543` | FAIL |
| Cap/middle/right draw rule | Rust builds non-overlapping segments and clips the final middle UV strip | gamemd draws left cap width `7`, one tiled middle destination from `x+7` with width `button_width-10`, then right cap width `10` at `x+button_width-10`; right cap overwrites the last 7 px of the middle dest and the tile phase is based on the full middle dest | FAIL |
| Button text rects | released matches; pressed stores `x+2`, `y+5`, `w=rect.w-2`, so right edge becomes `button_right` | released right edge is `button_left+width-2`; pressed left/top shift to `+2/+5` but right edge remains `button_left+width-2` | FAIL |

## Concrete 800x600 Outputs

Control rects now match retail:

| Button | Rust rect | gamemd rect |
|---|---:|---:|
| Start `0x617` | `(635,242,162,37)` | `(635,242,162,37)` |
| Choose `0x5AA` | `(635,286,162,37)` | `(635,286,162,37)` |
| Back `0x5C0` | `(644,535,156,42)` | `(644,535,156,42)` |

Art y still differs:

| Button | Rust released/pressed art y | gamemd released/pressed art y |
|---|---:|---:|
| Start `0x617` | `242 / 242` | `245 / 247` |
| Choose `0x5AA` | `286 / 286` | `289 / 291` |
| Back `0x5C0` | `535 / 535` | `541 / 543` |

Pressed text rect still differs by right edge/width:

| Button | Rust pressed text rect | gamemd pressed text rect |
|---|---:|---:|
| Start `0x617` | `(637,247,160,37)` | `(637,247,158,37)` |
| Choose `0x5AA` | `(637,291,160,37)` | `(637,291,158,37)` |
| Back `0x5C0` | `(646,540,154,42)` | `(646,540,152,42)` |

## Player-Visible Findings

1. **FAIL - Art strip is too high and does not press down.** Rust now keeps the 30 px PCX height, but it anchors at the control top. Retail centers the strip and moves it down 2 px while pressed. Player-visible difference: Start/Choose/Back sit too high inside their control rects, and pressed feedback lacks the retail art movement. Rust: `src/app_skirmish_shell_render.rs:347-384`. gamemd evidence: `OwnerDraw_Button_00612B70` vertical placement block.

2. **FAIL - Middle/right composition does not match retail.** Rust emits non-overlapping cap/middle/right segments. Retail paints a wider middle destination and lets the right cap overwrite 7 px, with `FUN_006BA3E0` tile phase based on that full destination. Player-visible difference: the right seam and repeated middle texture can be off even though the same PCX family is selected. Rust: `src/app_skirmish_shell_render.rs:280-319`. gamemd evidence: `OwnerDraw_Button_00612B70` cap calls plus `FUN_006BA3E0`.

3. **FAIL - Pressed text rect is 2 px too wide.** Rust shifts pressed text left to `x+2` but keeps width `rect.w-2`, which moves the right edge to the full button right. Retail keeps the right edge at `button_left + width - 2`. Player-visible difference: pressed labels center slightly differently from retail. Rust: `src/app_skirmish_shell_render.rs:1339-1352`. gamemd evidence: text rect setup in `OwnerDraw_Button_00612B70`.

## Adjacent Findings

- The older broad trace `traces/SKIRMISH_OWNER_DRAW_CONTROLS_VISUAL_STATES_TRACE.md` is stale for Start/Choose rects and native-height output; current Rust now matches those two pieces.
- `traces/SKIRMISH_OWNER_DRAW_BUTTON_PRESS_RELEASE_TRACE.md` correctly captured the vertical placement and cap/middle rule failures. Its prior pressed-text PASS should be treated as superseded by this narrower re-check of the right edge.
- Disabled Start validation visuals were intentionally not traced here because this scenario is released/pressed enabled states only.

## Verdict Tally

PASS: 5 | FAIL: 3 | UNCHECKED: 0 | NOT-IMPLEMENTED: 0
