# Skirmish Owner-Draw Button Pixel Recheck 800x600 - Ghidra Research Report

**Address(es):** `OwnerDraw_Button_00612B70 @ 0x00612B70`, `FUN_006BA3E0 @ 0x006BA3E0`, `FUN_00621040 @ 0x00621040`, `FUN_0060F9A0 @ 0x0060F9A0`, `FUN_0060B000 @ 0x0060B000`, `FUN_0060B350 @ 0x0060B350`, `FUN_006AE2C0 @ 0x006AE2C0`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** standard offline YR Skirmish dialog `0x102` at `800x600`, limited to Start Game `0x617`, Choose Map `0x5AA`, and Back `0x5C0` owner-draw button visual/input boundary: final rects, art vertical placement, pressed art movement, cap/middle/right composition and tile phase, text rect edges, enabled text color source, and current Rust status.
**Non-Scope:** Choose Map modal internals after the button action, Start launch validation modal internals, combo/listbox/checkbox/trackbar visuals, runtime screenshot capture, and changing Rust.
**Confidence:** High for binary control flow, geometry formulas, and current Rust status; Medium for exact source PCX dimensions where this report relies on prior asset mapping instead of re-extracting MIX contents.
**Active in YR:** Yes. `FUN_006AE2C0` is the live offline Skirmish setup loop, `FUN_0060F9A0` installs `OwnerDraw_Button_00612B70` for Button controls whose low style bits satisfy `(style & 0x0B) == 0x0B`, and resource/layout reports identify `0x617`, `0x5AA`, and `0x5C0` as those controls on dialog `0x102`.

## Superseded Asset-Family Correction - 2026-05-24

The PCX asset-family conclusion in this report is superseded for Start Game
`0x617`, Choose Map `0x5AA`, and Back `0x5C0`. A later recheck of the shell
classifier found that the active setup path sets these right-panel buttons to
owner-draw type `1`; the type-1 branch of `OwnerDraw_Button_00612B70` draws
`SDBTNANM.SHP` frames `2`/`4`, not the generic `bue_*30.pcx` / `bde_*30.pcx`
branch. Use
`SKIRMISH_RIGHT_PANEL_BUTTON_SDBTNANM_TYPE1_RECHECK_GHIDRA_REPORT.md` as the
current asset-family source for these three sidebar buttons.

## 0. Working Notes

**Target question:** At 800x600, what is the exact live owner-draw visual/input contract for Start Game `0x617`, Choose Map `0x5AA`, and Back `0x5C0`, and which older trace claims are now stale against current Rust?

**Non-goals:** no Choose Map modal internals beyond `ChooseMap` action dispatch; no Start validation modal content; no unrelated shell controls; no implementation patch.

**Evidence needed to mark COMPLETE:** read-only Ghidra confirmation for live Skirmish route, owner-draw install, 800x600 rect helpers, art state/placement/composition, middle tiler, text rect/color call; current Rust scan of layout, render, and mouse action boundary; stale-doc wording for contradicted docs.

**Stop conditions:** all scoped material findings have binary evidence plus Rust status, every open question is resolved or explicitly deferred, and no Rust/INI/in-repo docs are modified.

## 1. Overview

The three scoped buttons are live Win32 Button children on standard offline Skirmish dialog `0x102`. In YR they paint through `OwnerDraw_Button_00612B70`: released art uses `bue_*30.pcx`, pressed art uses `bde_*30.pcx`, the selected 30px art strip is vertically centered in the 42px control, pressed art moves down by 2px, labels are yellow and centered inside an inset edge rect, and disabled state forces released art plus a half-black alpha overlay.

Current Rust has already fixed several failures reported by older traces: the final 800x600 rects, native art height/y placement, pressed art y movement, text rect right/bottom semantics, and enabled text color now match the verified contract. The remaining current mismatch in this slice is the middle PCX source phase/crop: Rust draws the correct overlapping destination span, but starts the middle texture at source x=0 instead of using the binary tiler's centered source offset when the destination width is narrower than the PCX.

## 2. Key Verified Facts

| Finding | Evidence | Active in YR |
|---|---|---|
| Standard offline Skirmish reaches dialog loop `0x102`; loop exits on `0x617` Start or `0x5C0` Back. Choose Map is a command inside the dialog, not a loop exit. | `FUN_006AE2C0` decompile: `FUN_00622650(0)` creates the dialog, loop waits until `local_4 == 0x617 || local_4 == 0x5C0`, return is `local_4 == 0x617`. | Yes |
| `0x617`, `0x5AA`, `0x5C0` route to `OwnerDraw_Button_00612B70` by Button class plus low style bits `0x0B`. | `FUN_0060F9A0` decompile Button branch assigns `OwnerDraw_Button_00612B70` for `(style & 0x0B) == 0x0B`; `SKIRMISH_SHELL_LAYOUT_ASSETS_GHIDRA_REPORT.md` lists `0x617/0x5AA/0x5C0` as Button controls with style `0x5000000B`/`0x5000200B`. | Yes |
| Final 800x600 button rects are Start `(644,241,156,42)`, Choose `(644,283,156,42)`, Back `(644,535,156,42)`. | `FUN_0060B000` decompile uses `SDBTNANM` width/height and right-panel tile snap for Start/Choose; `FUN_0060B350` uses `SDBTNANM` and right-panel bottom formula for Back; `SKIRMISH_RESIZE_SHELL_CHILD_CONTROL_0060C0C0_COMPLETE_0X102_POLICY_GHIDRA_REPORT.md` computes these exact rects. | Yes |
| Art chooses 30-family PCX for these 42px controls, centers 30px art vertically, and adds +2px y while pressed. | `OwnerDraw_Button_00612B70` assembly `0x006132B9..0x006132F9` selects 24/30 table; `0x00613394..0x006133AE` computes `top + (height - art_height) / 2` and adds `2` when pressed. | Yes |
| Released/pressed PCX names are `bue_li/mi/ri30.pcx` and `bde_li/mi/ri30.pcx`; disabled forces released `'u'`, not `bud_*`. | `0x00613240..0x00613262` sets state char `'u'`/`'d'` and forces `'u'` under `WS_DISABLED`; `0x006133B2..0x006134DA` formats `b%c%c_li/mi/ri%d.pcx` with second char `'e'`. | Yes |
| Composition is left cap at x, middle at `x+7` with width `button_width-10`, right cap at `x+button_width-10` with width `10`, so right cap overwrites 7px of the middle. | `0x00613441` left blit, `0x0061348D..0x006134C4` middle rect/call, `0x0061351D..0x0061355D` right rect/blit. | Yes |
| Middle helper tiles/modulo-copies and centers the source crop when source is wider than destination. | `FUN_006BA3E0` forced decompile: `uVar5 = (src_width - dest_width)/2`, masked to zero when negative, then inner loop reads `(start_x + col) % src_width`; assembly call site `0x006134C4`. | Yes |
| Button text rect is released `left=x, top=y+1, right=x+w-2, bottom=y+h`; pressed changes only `left=x+2, top=y+5`, keeping right/bottom fixed. | `OwnerDraw_Button_00612B70` assembly `0x00613591..0x006135CD`; `SKIRMISH_TEXT_RENDERER_CALLER_PIXEL_CONTRACT_GHIDRA_REPORT.md`. | Yes |
| Enabled label color source is `DAT_00AC18A4 = 0x0000FFFF`, which `FUN_00621040` treats as yellow source RGB, not dark `0x00000C05`. | `FUN_0060F9A0` initializes `DAT_00AC18A4 = 0xFFFF`; `OwnerDraw_Button_00612B70` passes it to `FUN_00621040`; `FUN_00621040` decompile extracts low byte as R and second byte as G. | Yes |
| Call-site flags are effectively `0x05` (`h-center | v-center`); adjacent pushed `0x0C` is a dead/unused stack argument in the recovered signature. | `OwnerDraw_Button_00612B70` call sequence `0x006135D4..0x006135EE` pushes `0x0C`, then `0x5`, then color; `FUN_00621040` reads the `0x5` slot as flags and tests bit `0x04` for vertical centering. | Yes |

## 3. Current Rust Status

| Surface | Current state | Verdict |
|---|---|---|
| `src/ui/skirmish_shell/layout.rs` `compute_layout` | 800x600 tests expect Start `(644,241,156,42)`, Choose `(644,283,156,42)`, Back `(644,535,156,42)`. | Matches current rect evidence |
| `src/ui/skirmish_shell/state.rs` `hit_test_owner_draw_button` and `action_for_owner_draw_button` | Hit-test uses the same layout rects; `StartGame0x617 -> StartGame`, `ChooseMap0x5AA -> ChooseMap`, `Back0x5C0 -> BackOrExit`. | Matches action boundary except Back app policy differs from native process loop by design context |
| `src/app.rs` mouse down/up | Mouse-down arms `pressed_owner_draw_button`; mouse-up only dispatches if release is still inside same owner-draw button; Choose opens current modal, Start launches session, Back currently exits event loop. | Press/release boundary mostly matches; Back app policy remains parent-scoped follow-up |
| `src/app_skirmish_shell_render.rs` `button_art_y` / `push_button_30` | Uses native entry height and `top + (rect.h - art_h)/2 + 2 when pressed`; tests assert centered/pressed movement. | Matches art y and pressed y |
| `src/app_skirmish_shell_render.rs` `build_button_segments` | Emits left, middle span `rect.w - right_w` from `x+left_w`, and right cap at `x+rect.w-right_w`; this preserves the 7px right-cap overlap. | Destination composition matches |
| `src/app_skirmish_shell_render.rs` middle UV phase | For a partial middle span it scales UV width from source x=0; it does not apply `start_x = max(0, (src_w - dest_w)/2)` before sampling. For 156px buttons and prior `bue_mi30` width 177, binary starts around source x=15, Rust starts at 0. | Remaining mismatch |
| `src/app_skirmish_shell_render.rs` `button_text_rect` | Computes released `(x, y+1, w = right-2-x, h = bottom-y)` and pressed `(x+2, y+5, right/bottom fixed)`. | Matches text edge contract |
| `src/app_skirmish_shell_render.rs` `push_button_label_draw` | Uses `SHELL_LABEL_TEXT_RGB = [1,1,0]` and center+v-center flags. | Matches enabled yellow source color |

## 4. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Offline Skirmish loop/action boundary | verified | `FUN_006AE2C0` | Choose Map modal internals out of scope |
| Owner-draw install path | verified | `FUN_0060F9A0`; resource report | none |
| Start/Choose final rect helper | verified | `FUN_0060B000`; resize-policy report | runtime screenshot optional |
| Back final rect helper | verified | `FUN_0060B350`; resize-policy report | runtime screenshot optional |
| PCX state/name selection | verified | `0x00613240..0x006134DA` | none |
| Art y and pressed movement | verified | `0x00613394..0x006133AE` | none |
| Cap/middle/right destination composition | verified | `0x00613441`, `0x006134C4`, `0x0061355D` | none |
| Middle source phase/crop | verified | `FUN_006BA3E0`; prior PCX width doc | exact bde middle dimensions not freshly re-extracted |
| Text rect/color/flags | verified | `0x00613591..0x006135EE`; `FUN_00621040` | disabled color not in this enabled-button slice |
| Current Rust scan | verified | listed files/functions above | no patch made |

## 5. Open Questions - Final State

- `[RESOLVED] OQ1 - Are the scoped controls live in standard YR Skirmish? -> Yes, dialog 0x102 reaches these Button controls and owner-draw install selects OwnerDraw_Button_00612B70.` (evidence: `FUN_006AE2C0`, `FUN_0060F9A0`, `SKIRMISH_SHELL_LAYOUT_ASSETS_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ2 - Which final 800x600 rects should Rust use? -> Start `(644,241,156,42)`, Choose `(644,283,156,42)`, Back `(644,535,156,42)`; older `(635,242,162,37)` Start/Choose rows are stale.` (evidence: `FUN_0060B000`, `FUN_0060B350`, `SKIRMISH_RESIZE_SHELL_CHILD_CONTROL_0060C0C0_COMPLETE_0X102_POLICY_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ3 - Does art stretch to 42px? -> No, 30-family art remains native height and is centered in the control.` (evidence: `0x006132B9..0x006133A3`)
- `[RESOLVED] OQ4 - Does pressed state move art? -> Yes, art y adds +2 only when pressed and not disabled.` (evidence: `0x006133A1..0x006133AE`)
- `[RESOLVED] OQ5 - Does middle/right composition still differ? -> Destination overlap now matches Rust, but source phase still differs because binary centers the source crop in FUN_006BA3E0.` (evidence: `0x0061348D..0x006134C4`, `FUN_006BA3E0`, Rust `build_button_segments`)
- `[RESOLVED] OQ6 - What are the text edge semantics? -> Released inset top+1/right-2; pressed left+2/top+5 with fixed right/bottom.` (evidence: `0x00613591..0x006135CD`)
- `[RESOLVED] OQ7 - What is enabled text color? -> `DAT_00AC18A4 = 0x0000FFFF`, interpreted as yellow source RGB by FUN_00621040.` (evidence: `FUN_0060F9A0`, `FUN_00621040`, `FUN_00621040_RGB_BYTE_PERMUTATION_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ8 - Does Choose Map button investigation require modal internals? -> No; scoped boundary is action dispatch. Current Rust opens `ChooseMapModalState`; modal internals belong to slot 1.` (evidence: `src/app.rs` `open_choose_map_modal`)
- `[DEFERRED] OQ9 - Pixel-perfect retail screenshot diff for the centered middle source phase.` (category: `needs-runtime-debugger`; reason: static binary proves formula but final visual screenshot would quantify the seam; next-step-if-pursued: capture 800x600 retail shell and compare the middle strip crop against Rust)
- `[DEFERRED] OQ10 - Exact pressed/down middle PCX dimensions from retail assets.` (category: `bounded-cost-too-high`; reason: prior asset docs cover `bue_mi30` width, and binary formula is state-independent; next-step-if-pursued: extract all six PCXs from MIX and record dimensions/checksum)

## 6. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Middle PCX helper starts sampling at `max(0, (src_w - dest_w)/2)` and wraps modulo across source width/height. | `FUN_006BA3E0` decompile; call `0x006134C4`; prior `bue_mi30.pcx = 177x30` doc | mismatch | `src/app_skirmish_shell_render.rs` `build_button_segments` / middle UV construction | Preserve current destination overlap, but offset middle UV origin for centered source crop when `dest_w < src_w`. | 800x600 Start/Choose/Back middle strip begins at the same source phase as retail; proposed test `skirmish_button_middle_tile_uses_centered_source_phase_800`. | Do not remove the 7px right-cap overlap while fixing phase. |
| Start/Choose/Back 800x600 rects are `644,241/283/535,156x42`; they are not generic right-anchor `162x37` controls. | `FUN_0060B000`, `FUN_0060B350`; resize-policy report | none observed | `src/ui/skirmish_shell/layout.rs`, `state.rs` hit-test | Keep current rects and tests. | `hit_test_owner_draw_button(643,241)` misses Start; `(644,241)` hits Start; proposed test `skirmish_ownerdraw_buttons_keep_snap_rects_800`. | Do not revive stale `(635,242,162,37)` / `(635,286,162,37)` Start/Choose rects. |
| Text rect/color now match: yellow `0x0000FFFF`, fixed right/bottom, pressed left+2/top+5. | `0x00613591..0x006135EE`; `FUN_00621040`; Rust scan | none observed | `src/app_skirmish_shell_render.rs` `button_text_rect`, `push_button_label_draw` | Keep current text rect and yellow color. | Press Start: text moves about +1px right/+2px down, not +2/+4; proposed test `skirmish_button_pressed_text_keeps_binary_right_bottom_edges`. | Do not use dark `0x00000C05` for enabled buttons; do not treat pushed `0x0C` as the alignment flags. |

## 7. Negative Facts / Do Not Do

- Do not use stale generic-right-anchor Start/Choose rects `(635,242,162,37)` and `(635,286,162,37)`. Active in YR: No for standard owner-draw Button metadata. Evidence: `ResizeShellChildControl_0060C0C0` branch order documented in `SKIRMISH_RESIZE_SHELL_CHILD_CONTROL_0060C0C0_COMPLETE_0X102_POLICY_GHIDRA_REPORT.md`, with `FUN_0060B000` as the live helper.
- Do not stretch cap/middle/right art to the full 42px HWND height. Active in YR: No. Evidence: 30-family size table and vertical centering at `0x006132B9..0x006133AE`.
- Do not replace disabled/default art with `bud_*` for these buttons. Active in YR: No on this path. Evidence: disabled branch at `0x00613254..0x00613262` forces the state char back to `'u'`, then alpha overlay applies later.
- Do not invent hover/focus PCX changes for these default PCX buttons. Active in YR: No for this path. Evidence: `OwnerDraw_Button_00612B70` has timer/custom-message state, but default filename path uses pressed bit and disabled style only.
- Do not use `0x00000C05` as the enabled Start/Choose/Back label color. Active in YR: No for enabled buttons. Evidence: `DAT_00AC18A4 = 0xFFFF` initialization in `FUN_0060F9A0` and pass-through at `0x006135E1`.

## 8. Stale Docs / Follow-Up Docs

- `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_OWNERDRAW_BUTTON_PIXEL_LAYOUT_GHIDRA_REPORT.md` replacement wording: "For standard offline Skirmish `0x102` at 800x600, Start `0x617` and Choose Map `0x5AA` use the `FUN_0060B000` owner-draw button snap rects `(644,241,156,42)` and `(644,283,156,42)`, while Back `0x5C0` uses `FUN_0060B350` `(644,535,156,42)`. Current Rust now matches these rects, native 30px art y centering/pressed +2 movement, fixed-right/bottom text rects, and yellow enabled label color. Remaining current mismatch: middle PCX source phase should use `FUN_006BA3E0`'s centered crop offset before modulo tiling."
- `C:/Users/enok/Documents/ra2-rust-game-docs/traces/SKIRMISH_START_CHOOSE_BACK_OWNER_DRAW_BUTTONS_800X600_TRACE.md` replacement wording: "Superseded for current Rust status by `SKIRMISH_OWNERDRAW_BUTTON_PIXEL_RECHECK_800X600_GHIDRA_REPORT.md`: art y, pressed art movement, button rects, text rects, and enabled text color are no longer failing in current Rust; only middle PCX source phase remains a scoped visual mismatch."
- `C:/Users/enok/Documents/ra2-rust-game-docs/traces/SKIRMISH_BUTTON_TEXT_RECTS_PRESSED_OFFSETS_800X600_TRACE.md` replacement wording: "Superseded for current Rust status by `SKIRMISH_OWNERDRAW_BUTTON_PIXEL_RECHECK_800X600_GHIDRA_REPORT.md`: current `button_text_rect` keeps binary right/bottom edges and `push_button_label_draw` uses yellow enabled text. Keep the binary facts, but remove the current-Rust FAIL verdicts."

## Sources

- Ghidra read-only: `OwnerDraw_Button_00612B70 @ 0x00612B70`, `FUN_006BA3E0 @ 0x006BA3E0`, `FUN_00621040 @ 0x00621040`, `FUN_0060F9A0 @ 0x0060F9A0`, `FUN_0060B000 @ 0x0060B000`, `FUN_0060B350 @ 0x0060B350`, `FUN_006AE2C0 @ 0x006AE2C0`, `FUN_00622650 @ 0x00622650`.
- Assembly contexts: `0x00613240..0x00613262`, `0x006132B9..0x006132F9`, `0x00613394..0x006133AE`, `0x00613441`, `0x0061348D..0x006134C4`, `0x0061351D..0x0061355D`, `0x00613591..0x006135EE`, `0x006135F3..0x0061361B`.
- Prior docs referenced: `SKIRMISH_RESIZE_SHELL_CHILD_CONTROL_0060C0C0_COMPLETE_0X102_POLICY_GHIDRA_REPORT.md`, `SKIRMISH_SHELL_LAYOUT_ASSETS_GHIDRA_REPORT.md`, `SKIRMISH_TEXT_RENDERER_CALLER_PIXEL_CONTRACT_GHIDRA_REPORT.md`, `SHELL_BUTTON_PAINT_DETAILS_GHIDRA_REPORT.md`, `FUN_00621040_RGB_BYTE_PERMUTATION_GHIDRA_REPORT.md`, `SHELL_PCX_BUTTON_TILE_AND_CAP_GEOMETRY_GHIDRA_REPORT.md`.
- Rust read-only scan: `src/ui/skirmish_shell/layout.rs`, `src/ui/skirmish_shell/state.rs`, `src/app_skirmish_shell_render.rs`, `src/app.rs`, `src/render/skirmish_shell_chrome.rs`.
