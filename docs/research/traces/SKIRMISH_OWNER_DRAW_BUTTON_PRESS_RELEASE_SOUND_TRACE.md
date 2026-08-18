# Skirmish Owner-Draw Button Press/Release Sound Trace

Date: 2026-05-23

Scenario: standard/dev Skirmish dialog `0x102` at `800x600`; mouse down then mouse up on each owner-draw button `0x617` Start Game, `0x5AA` Choose Map, and `0x5C0` Back. Scope is limited to hit gating, pressed visual state/offsets, action dispatch boundary, and the two native click sound timings.

Ghidra use: none in this run. Binary facts are taken from existing verified read-only reports named below. No Rust, INI, or in-repo docs were modified.

## Evidence Base

- Current Rust read-only scan:
  - `src/ui/skirmish_shell/layout.rs`
  - `src/ui/skirmish_shell/state.rs`
  - `src/app.rs`
  - `src/app_skirmish_shell_render.rs`
  - `src/rules/ruleset.rs`
- Verified research reports:
  - `skirmish-ui/SKIRMISH_0X102_COMPLETE_CHILD_RECT_MATRIX_GHIDRA_REPORT.md`
  - `skirmish-ui/SKIRMISH_BUTTON_CLICK_SOUND_PARITY_GHIDRA_REPORT.md`
  - `skirmish-ui/SKIRMISH_OWNERDRAW_PUSH_BUTTON_SOUNDS_GHIDRA_REPORT.md`
  - `skirmish-ui/SKIRMISH_START_GAME_HANDOFF_SESSION_PACKING_GHIDRA_REPORT.md`
  - `skirmish-ui/SKIRMISH_START_VALIDATION_FAILURE_UI_GHIDRA_REPORT.md`
  - `skirmish-ui/SKIRMISH_TEXT_RENDERER_CALLER_PIXEL_CONTRACT_GHIDRA_REPORT.md`
  - `traces/SKIRMISH_START_CHOOSE_BACK_OWNER_DRAW_BUTTONS_800X600_TRACE.md` with stale Rust deltas rechecked against current source.
- INI:
  - `ini/rulesmd.ini:643` has `GUIMainButtonSound=MenuClick`.
  - `ini/rulesmd.ini:703` has `GenericClick=MenuClick`.

## Pipeline

Rust path:

`WindowEvent::MouseInput` -> `handle_skirmish_shell_mouse_down` -> `hit_test_owner_draw_button` -> `pressed_owner_draw_button` -> immediate `GUIMainButtonSound` -> render transition `GenericClick` -> `push_button_30` / `button_text_rect` -> `handle_skirmish_shell_mouse_up` -> same-button release gate -> `action_for_owner_draw_button` -> `handle_skirmish_shell_action`.

gamemd path:

standard offline Skirmish launcher -> dialog `0x102` -> `FUN_0060F9A0` subclasses `Button` controls with style `(style & 0x0B) == 0x0B` to `OwnerDraw_Button_00612B70` -> `WM_LBUTTONDOWN` plays `GUIMainButtonSound` -> default Button state/capture -> `WM_PAINT` can play `GenericClick` on `'u' -> 'd'` -> default Button emits notification `0` on activation -> `FUN_006AE3F0` `WM_COMMAND` -> `FUN_006ACEE0`.

## Stage Results

| Stage | Rust output | gamemd output | Verdict |
|---|---|---|---|
| Active standard-YR route | `OwnerDrawButton::{StartGame0x617, ChooseMap0x5aa, Back0x5c0}` in `state.rs:37-40`; app path gated by `dev_skirmish_shell_enabled` | standard offline YR creates dialog `0x102`, routes the three buttons to `OwnerDraw_Button_00612B70`; active, not TS legacy | PASS |
| Final 800x600 button rects | Start `(644,241,156,42)`, Choose `(644,283,156,42)`, Back `(644,535,156,42)` from `layout.rs:566-572` | same final rects in complete child matrix report for 800x600 | PASS |
| Hit gating edge rule | `RectPx::contains` accepts `x>=left`, `y>=top`, `x<right`, `y<bottom`; e.g. Start accepts `[644,800) x [241,283)` | child window/client hit region is the same final rect; no owner-draw override changes the hit area | PASS |
| Press identity on mouse down | app stores the exact enum from `hit_test_owner_draw_button` at `app.rs:757-764` | default Button proc sets the pressed bit for the receiving child HWND after `WM_LBUTTONDOWN` | PASS |
| Mouse-down sound timing | after the hit test and before release/action, app calls `play_main_menu_button_sound` at `app.rs:757-764`, resolving `GUIMainButtonSound` at `app.rs:874-880` | `OwnerDraw_Button_00612B70` plays Rules `+0x188` / `GUIMainButtonSound` on `WM_LBUTTONDOWN` before command dispatch | PASS |
| Mouse-down sound ID | stock Rust rules resolve `GUIMainButtonSound=MenuClick` from `rulesmd.ini:643` | stock YR Rules `+0x188` is `MenuClick` | PASS |
| Paint-transition sound timing | first render with a pressed button calls `update_owner_draw_button_paint_sound` before building the visual batch at `app_skirmish_shell_render.rs:2047-2052` and `:2085` | `WM_PAINT` plays Rules `+0x70C` / `GenericClick` only when current state is down and prior rendered state was up | PASS |
| Paint-transition sound ID | `play_skirmish_shell_generic_click_sound` resolves `GenericClick` via `app.rs:891-927`; stock `rulesmd.ini:703` is `MenuClick` | stock YR Rules `+0x70C` is `MenuClick` | PASS |
| Released/pressed PCX family | `button_entries` chooses `bue_li30/bue_mi30/bue_ri30` released and `bde_li30/bde_mi30/bde_ri30` pressed at `app_skirmish_shell_render.rs:251-271` | `OwnerDraw_Button_00612B70` formats the same `u`/`d`, `30` owner-draw PCX families | PASS |
| Pressed art y offset | 30 px art is centered in the 42 px rect and moves down 2 px when pressed: Start `247/249`, Choose `289/291`, Back `541/543` via `button_art_y` at `app_skirmish_shell_render.rs:330-338` | same formula from button pixel/layout reports: `top + (42-30)/2`, plus `+2` pressed | PASS |
| Cap/middle/right geometry | current `build_button_segments` for width `156` emits left `7`, middle from `x+7` to `x+153`, right at `x+146`, producing the retail 7 px overlap; test at `app_skirmish_shell_render.rs:2324-2338` | gamemd draws middle width `button_width - 10` from `x+7`, then right cap width `10` at `x+button_width-10` | PASS |
| Button text rect/pressed offset | `button_text_rect` keeps right/bottom fixed: released `(644,242,154,41)`, pressed `(646,246,152,37)` for a 156x42 rect at `app_skirmish_shell_render.rs:1490-1503` | `OwnerDraw_Button_00612B70` shifts pressed left/top by `+2/+5` while preserving right/bottom | PASS |
| Button text color | button labels now use `SHELL_LABEL_TEXT_RGB = [1,1,0]` through `push_button_label_draw` at `app_skirmish_shell_render.rs:1470-1487` | verified button path uses the yellow shell text source before display conversion | PASS |
| Release-inside action gate | mouse up takes the stored pressed button and only dispatches if release hit matches at `app.rs:785-793`; mismatched release clears state and does not dispatch | default Button activation emits notification `0` only for an activated button, then parent command handling runs | PASS |
| Control-id to action mapping | Start -> `StartGame`, Choose -> `ChooseMap`, Back -> `BackOrExit` at `state.rs:1379-1384` | `FUN_006ACEE0` branches on `0x617`, `0x5AA`, and `0x5C0` | PASS |
| Start successful launch packing | Rust builds a `SkirmishLaunchSession` and starts it at `app.rs:583-592`; exact packed values for all controls were not recomputed against gamemd in this run | gamemd disables Start, validates, packs live controls/session/node globals, clears preview, then writes result `0x617` | UNCHECKED |
| Choose Map full modal behavior | Rust opens `ChooseMapModalState` at `app.rs:597-618`; exact modal hide/show, result, preview refresh, and modal-button owner-draw behavior were not recomputed in this run | gamemd `0x5AA` saves state, hides setup, runs dialog `0x6B`, then restores/commits based on result | UNCHECKED |
| Back action effect | Rust calls `event_loop.exit()` at `app.rs:594-595` | gamemd writes/returns dialog result `0x5C0`; `FUN_006AE2C0` returns false to the shell flow, not process exit | FAIL |

## Failures

1. **Back exits the program instead of cancelling/leaving Skirmish setup.** A normal Back click is much more destructive in Rust than in retail. Retail uses `0x5C0` as the Skirmish dialog cancel result and returns false to the caller; current Rust calls `event_loop.exit()`.

## Unchecked

- Start successful packing is still not a PASS here because this slot did not recompute every Rust `SkirmishLaunchSession` field against the verified `FUN_006ACEE0` control packing and result write.
- Choose Map full modal result behavior is not a PASS here because this slot only verified dispatch into modal state, not every modal `0x6B` visible/control side effect.
- Retail runtime audibility of the two sound call sites remains dependent on Windows paint/message timing. The binary has both sound sites and current Rust has both corresponding requests; an audio runtime capture would be needed to prove the player always hears two distinct clicks on every physical press.

## Adjacent Findings

- Start validation failure UI remains incomplete in Rust per `SKIRMISH_START_VALIDATION_FAILURE_UI_GHIDRA_REPORT.md`; that is adjacent because this scenario does not force an invalid Start click.
- Choose Map modal button press/release sounds should reuse the same owner-draw push-button sound contract when tracing dialog `0x6B`; this run is scoped to parent dialog `0x102` buttons only.
- Older traces that say Rust lacked Skirmish button sounds, vertical art centering, cap overlap, or fixed-right text rects are stale for the current worktree.

## Verdict Tally

PASS: 15 | FAIL: 1 | UNCHECKED: 2 | NOT-IMPLEMENTED: 0

## Status

COMPLETE
