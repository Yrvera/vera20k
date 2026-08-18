# Skirmish Owner-Draw Button Press/Release Trace

Date: 2026-05-22

Scenario: native/dev Skirmish shell dialog `0x102` at `800x600`; click and hold a visible owner-draw shell button, using Start `0x617` and Back `0x5C0` as the concrete checked buttons, then release inside the same button. Compare pressed visual state, release action, and click-sound ordering.

Scope is limited to Start/Back owner-draw button press/release behavior. Choose Map modal behavior, Start launch packing, and validation modal contents are adjacent unless they directly affect the button press/release path.

Ghidra use was read-only. Live spot-check decompiled `OwnerDraw_Button_00612B70`, `FUN_006ae3f0`, and `FUN_006acee0` by function name only; no mutating Ghidra tools were used.

## Evidence Base

- Rust read-only scan: `C:/Users/enok/Documents/ra2-rust-game/src/ui/skirmish_shell/state.rs`, `C:/Users/enok/Documents/ra2-rust-game/src/ui/skirmish_shell/layout.rs`, `C:/Users/enok/Documents/ra2-rust-game/src/app_skirmish_shell_render.rs`, `C:/Users/enok/Documents/ra2-rust-game/src/app.rs`.
- Verified docs: `SKIRMISH_BUTTON_CLICK_SOUND_PARITY_GHIDRA_REPORT.md`, `SKIRMISH_OWNERDRAW_BUTTON_PIXEL_LAYOUT_GHIDRA_REPORT.md`, `SKIRMISH_SHELL_VIEWPORT_ORIGIN_FOLLOWUP_GHIDRA_REPORT.md`, `SKIRMISH_START_GAME_HANDOFF_SESSION_PACKING_GHIDRA_REPORT.md`, `SKIRMISH_START_VALIDATION_FAILURE_UI_GHIDRA_REPORT.md`, `SKIRMISH_SUBCLASS_THUNK_00610CA0_NON_TEXT_BEHAVIOR_GHIDRA_REPORT.md`.
- Geometry cross-check: `SHELL_PCX_BUTTON_TILE_AND_CAP_GEOMETRY_GHIDRA_REPORT.md`; its main-menu active-YR header is stale, but the PCX-branch geometry is confirmed for Skirmish by the Skirmish button pixel-layout report.
- INI: `ini/rulesmd.ini:643` has `GUIMainButtonSound=MenuClick`; `ini/rulesmd.ini:703` has `GenericClick=MenuClick`.

## Pipeline

Rust path:

`WindowEvent::MouseInput` -> `handle_skirmish_shell_mouse_down` -> `hit_test_owner_draw_button` -> `pressed_owner_draw_button` -> render `push_button_30` and `button_text_rect` -> `handle_skirmish_shell_mouse_up` -> same-button release check -> `action_for_owner_draw_button` -> `handle_skirmish_shell_action`.

gamemd path:

standard offline Skirmish launcher -> dialog `0x102` -> common shell child subclassing -> `OwnerDraw_Button_00612B70` for Start/Back buttons -> `WM_LBUTTONDOWN` sound/default Button processing -> `WM_PAINT` pressed art/text and possible transition sound -> default Button command -> `FUN_006ae3f0` `WM_COMMAND` -> `FUN_006acee0`.

## Stage Results

| Stage | Rust output | gamemd output | Verdict |
|---|---:|---:|---|
| Active standard-YR route | dev/native shell path has `OwnerDrawButton::{StartGame0x617, Back0x5c0}` and action mapping in `state.rs:37-48`, `state.rs:1259-1264` | dialog `0x102` creates Start `0x617` and Back `0x5C0`; common setup routes low-style `0x0B` buttons to `OwnerDraw_Button_00612B70`; live decompile confirms the active proc path | PASS |
| 800x600 control rects | Start `(635,242,162,37)`, Back `(644,535,156,42)` from `layout.rs:364-448` and tests at `layout.rs:545-549` | same final rects in viewport-origin report; Back uses `SDBTNANM.SHP=156x42` and right-panel tile anchor | PASS |
| Press hit identity | mouse-down stores the exact hit enum in `pressed_owner_draw_button` at `app.rs:592-621` | Button child receives `WM_LBUTTONDOWN`; default Button proc owns pressed bit consumed by `OwnerDraw_Button_00612B70` | PASS for inside-control press identity |
| Mouse-down click sound | no call to `play_sound` in `handle_skirmish_shell_mouse_down`; only main-menu shell calls `play_main_menu_button_sound` at `app.rs:670-688` | `OwnerDraw_Button_00612B70` handles `WM_LBUTTONDOWN`/`WM_LBUTTONDBLCLK`, plays `[AudioVisual] GUIMainButtonSound` (`MenuClick`), then calls previous WndProc | FAIL |
| Paint-transition click sound | renderer changes visual state but has no `GenericClick` transition sound state | first enabled paint transition from global last state `'u'` to current `'d'` can play `[AudioVisual] GenericClick` (`MenuClick`) before command handling | FAIL |
| Pressed PCX family | `push_button_30` selects down entries when `pressed_owner_draw_button == Some(button)` at `app_skirmish_shell_render.rs:1175-1198`; asset names are `bde_li30/mi30/ri30` | pressed state char `'d'` formats `bde_li30.pcx`, `bde_mi30.pcx`, `bde_ri30.pcx` for the default Skirmish PCX path | PASS |
| PCX art vertical position | Start/Back art is emitted at `rect.y` with native entry height at `app_skirmish_shell_render.rs:347-384`: Start y `242`, Back y `535`, pressed y unchanged | art y is `top + (height - 30) / 2`, plus `+2` while pressed: Start released `245`, pressed `247`; Back released `541`, pressed `543` | FAIL |
| Middle/cap horizontal composition | Rust uses non-overlapping spans: left, then middle total width `button_width - left_w - right_w`, then right at `rect.x + rect.w - right_w` (`app_skirmish_shell_render.rs:280-319`) | gamemd draws left width `7`, middle dest width `button_width - 10` from `x+7`, then right width `10` from `x+button_width-10`; right cap overwrites 7 px of middle | FAIL |
| Text pressed rect | released text rect `(x,y+1,w-2,h)`, pressed `(x+2,y+5,w-2,h)` at `app_skirmish_shell_render.rs:1319-1352`; Start released `(635,243,160,37)`, pressed `(637,247,160,37)` | same text rect and center/v-center flags from `OwnerDraw_Button_00612B70` / `FUN_00621040` | PASS |
| Release-inside activation | Rust fires only when pressed identity equals release identity at `app.rs:624-640` | default Button processing emits `WM_COMMAND`; `FUN_006acee0` handles Start/Back only for notification `0` | PASS for release-inside activation gate |
| Back action result | Back maps to `BackOrExit`, then calls `event_loop.exit()` at `app.rs:582-583` | Back `0x5C0` stores a cancel/back dialog result; `FUN_006AE2C0` returns false to the shell flow, not process exit | FAIL |
| Start failure disabled visual | Rust logs a `launch_session` error and has no Start disable/re-enable visual branch in this path | Start `0x617` disables the Start button before validation, shows failure UI on validation failures, then re-enables Start; disabled owner-draw uses released art plus alpha `0x80` overlay | NOT-IMPLEMENTED |
| Drag-off / mismatched release negative case | source suggests stored-identity mismatch cancels action and clears pressed state | default Win32 Button capture/release behavior should cancel command when release is not an activation, but this exact negative runtime was not re-simulated here | UNCHECKED |
| Audible double-sound runtime result | no Rust sound in this Skirmish path | binary has mouse-down and paint-transition sound call sites; whether retail always produces two audible clicks per press needs runtime capture because paint/message coalescing can affect the second call | UNCHECKED |

## Failures

1. **No Skirmish mouse-down click sound.** Pressing Start or Back is silent in the native/dev Skirmish shell path. Retail plays `GUIMainButtonSound=MenuClick` immediately on `WM_LBUTTONDOWN` before the command action.

2. **No Skirmish paint-transition click sound state.** Retail has a second conditional `GenericClick=MenuClick` call when the owner-draw paint first observes a released-to-pressed transition. Rust has no equivalent transition tracker in the Skirmish button renderer.

3. **Pressed art is vertically anchored wrong.** Rust draws the 30 px PCX strip at the control top and does not move it while pressed. Retail centers the 30 px strip inside the 37/42 px control and shifts it 2 px down while held. Start should draw at y `245` released and `247` pressed, not y `242` for both.

4. **Middle/cap composition lacks the 7 px right-cap overdraw.** Rust avoids overlap and crops the middle to the visible span. Retail draws a wider middle span and then overwrites 7 px with the right cap. The seam/texture phase can differ on every Start/Back press.

5. **Back action exits the app.** Retail Back produces dialog result `0x5C0` and returns false to the shell flow. Rust calls `event_loop.exit()`, so the player leaves the program instead of cancelling the Skirmish setup screen.

## Not Implemented

- **Start disabled/re-enable visual during validation.** Retail disables Start before validation and re-enables it on failure, producing a visible disabled owner-draw state. Rust has no equivalent disabled branch for this Skirmish Start path.

## Adjacent Findings

- Choose Map action remains a separate trace target. This report did not trace `0x5AA` beyond shared owner-draw button mechanics.
- Full Start Game packing and post-launch consumers are out of scope; prior reports cover them.
- Current Rust has fixed older stale findings for Start/Choose button rects and button text rects. Do not use stale reports that still say those two are mismatched without rechecking current source.

## Verdict Tally

PASS: 6 | FAIL: 5 | UNCHECKED: 2 | NOT-IMPLEMENTED: 1

## Status

COMPLETE
