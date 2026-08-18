# Skirmish Standard Route To Pixel Shell 800x600 Trace

Scenario: from the normal Main Menu path at `800x600`, open Skirmish setup with default settings and verify whether the player reaches the retail-shaped offline Skirmish dialog `0x102` pixel shell or the legacy egui setup.

## Verdict

PASS: 2 | FAIL: 2 | UNCHECKED: 1 | NOT-IMPLEMENTED: 0

Standard YR reaches the fullscreen-hosted dialog resource `0x102` directly through the normal offline Skirmish launcher. Current Rust does not route the normal player path to the pixel Skirmish shell. It sets `main_menu_show_skirmish_setup = true` from the main menu shell action and renders `draw_main_menu_with_maps` in egui unless `dev_skirmish_shell_enabled` is enabled. The recovered pixel shell is therefore present only as a dev/experimental path, not as the standard route.

## Pipeline

`Main Menu Skirmish/Single Player selection -> Skirmish setup launcher -> 800x600 shell host/layout -> first setup frame -> player sees setup UI`

Standard YR active path, verified read-only in Ghidra and prior audited reports:

`Main_Game -> FUN_006AE2C0 -> FUN_0072CF40 -> FUN_00622650(dialog 0x102/proc 0x006AE3F0) -> FUN_00622800 -> FUN_00623120 pump -> FUN_00622720`.

Current Rust standard path:

`main menu shell mouse_up -> MainMenuShellAction::SinglePlayer -> main_menu_show_skirmish_setup = true -> GameScreen::MainMenu render branch -> render_egui_main_menu_fallback -> main_menu::draw_main_menu_with_maps`.

## Stage Table

| Stage | Boundary Output | gamemd 800x600 | Rust 800x600 | Verdict |
|---:|---|---|---|---|
| 1 | Standard entry action | The normal offline Skirmish path calls `FUN_006AE2C0`; that function loads Skirmish background resources, creates/shows dialog `0x102`, pumps until Start `0x617` or Back `0x5C0`, and returns Start as `true`. | Main menu shell `SinglePlayer0x683` maps to `MainMenuShellAction::SinglePlayer`; app handling sets `main_menu_show_skirmish_setup = true` instead of entering `skirmish_shell_state` / `render_skirmish_shell`. | FAIL |
| 2 | Default render route after entry | Dialog `0x102` is shown with `ShowWindow(hwnd,1)` and the fullscreen host resizes it to `(0,0,g_ScreenWidth,g_ScreenHeight)`. At `800x600`, the player sees the native shell window at origin `(0,0)` covering `800x600`. | `GameScreen::MainMenu` calls `render_skirmish_shell` only when `dev_skirmish_shell_enabled` is true. The default route with `main_menu_show_skirmish_setup = true` calls `render_egui_main_menu_fallback`, which draws the old egui setup. | FAIL |
| 3 | Pixel shell 800x600 layout if forced on | Dialog `0x102` enters the fullscreen shell-host path; selected right-panel children such as `0x617`, `0x5AA`, `0x468`, and Back `0x5C0` use the verified 800x600 formulas. | `compute_layout(800,600)` encodes matching checked key rects: Start `(644,241,156,42)`, Choose Map `(644,283,156,42)`, Preview `(644,37,144,112)`, Back `(644,535,156,42)`. | PASS |
| 4 | Pixel shell checked first-paint chrome if forced on | Prior first-paint trace verifies the checked chrome slice: `MnScrnLCoopGameSetup` for width `800`, right-panel rects, no first-paint frame-10 overlay, top-clipped `SDBTM`, lower strip, and parent-before-child order. | Current `app_skirmish_shell_render.rs` keeps the same checked decisions: parent background role for `800`, disabled frame-10 overlay, top-clipped `SDBTM`, large lower strip, and parent/chrome order before controls. | PASS |
| 5 | Aggregate visible framebuffer equality | Not captured in this slot. | Not captured in this slot. | UNCHECKED |

## Failures

### Stage 1 - Standard entry action routes to egui setup

Player-visible difference: using the normal menu path does not land on the retail-shaped Skirmish setup shell. The player sees the non-retail egui setup surface instead of dialog `0x102`.

Rust evidence: `src/ui/main_menu_shell/state.rs:44` maps `SinglePlayer0x683` to `MainMenuShellAction::SinglePlayer`; `src/app.rs:945` handles that action; `src/app.rs:955` sets `state.main_menu_show_skirmish_setup = true`.

Gamemd evidence: read-only Ghidra decompile of `FUN_006AE2C0 @ 0x006AE2C0` shows the standard offline Skirmish launcher calls `FUN_0072CF40`, creates a dialog via `FUN_00622650`, stores the HWND in `DAT_00B0B59C`, calls `FUN_00622800`, pumps until result `0x617` or `0x5C0`, then returns `local_4 == 0x617`. `SKIRMISH_SHELL_VIEWPORT_ORIGIN_GHIDRA_REPORT.md` records the call-site bytes selecting dialog proc `0x006AE3F0`, resource id `0x102`, and `FUN_00622650`. This is active standard YR offline Skirmish, not TS legacy.

### Stage 2 - Default render branch does not draw the pixel shell

Player-visible difference: even though Rust has a pixel-shell renderer, it is not the standard route. Normal `GameScreen::MainMenu` rendering uses the pixel shell only behind the dev flag; otherwise the setup branch falls through to egui.

Rust evidence: `src/app.rs:151` documents `dev_skirmish_shell_enabled` as opt-in; `src/app.rs:416` reads the environment flag; `src/app.rs:1537` calls `render_skirmish_shell` only when the flag is true; `src/app.rs:1568` calls `render_egui_main_menu_fallback` otherwise; `src/ui/main_menu.rs:1` describes the egui skirmish setup screen.

Gamemd evidence: read-only Ghidra decompile of `FUN_00622800 @ 0x00622800` shows `ShowWindow(hwnd,1)` and `SetForegroundWindow(hwnd)`. Read-only Ghidra decompile of `FUN_0060C4A0 @ 0x0060C4A0` shows `MoveWindow(hwnd,0,0,g_ScreenWidth,g_ScreenHeight,0)` followed by child enumeration. `SKIRMISH_SHELL_VIEWPORT_ORIGIN_GHIDRA_REPORT.md` verifies dialog `0x102` is in the fullscreen shell-host allowlist and therefore covers the shell client/backbuffer. This path is active standard YR offline Skirmish.

## Checked Passes

Stage 3 passes only for the forced pixel shell path, not for standard reachability. Current layout tests and code produce the checked native `800x600` rects: Start `(644,241,156,42)`, Choose Map `(644,283,156,42)`, Preview `(644,37,144,112)`, and Back `(644,535,156,42)`. Rust evidence: `src/ui/skirmish_shell/layout.rs:378` and assertions at `src/ui/skirmish_shell/layout.rs:567`.

Stage 4 passes for the checked first-paint chrome slice when the dev shell is forced on. Rust evidence: `src/app_skirmish_shell_render.rs:1130`, `src/app_skirmish_shell_render.rs:1211`, `src/app_skirmish_shell_render.rs:1261`, and `src/app_skirmish_shell_render.rs:1299`. Gamemd evidence is the prior current trace `docs/research/traces/SKIRMISH_SHELL_CHROME_FIRST_PAINT_800X600_VISUAL_TRACE.md`.

## Unchecked

Full-frame pixel equality is unchecked. This slot did not launch retail YR, capture a retail framebuffer, launch Rust, or diff screenshots. Per the trace-action rule, no aggregate visual PASS is claimed without both framebuffers.

## Adjacent Findings

Main menu nomenclature and whether Rust's current `SinglePlayer` action is the final intended Skirmish entry affordance are adjacent to this slot. Choose Map, player-name edit focus, random map generation, full child control matrix, and post-Start launch/spawn behavior are also out of scope here.

## Sources

- Read-only Ghidra: `FUN_006AE2C0`, `FUN_0060C4A0`, `FUN_00622800`, `FUN_0072CF40`, `FUN_0060C540`, `FUN_00608CD0`, `FUN_00609730`, `FUN_0060B1D0`.
- `docs/research/skirmish-ui/SKIRMISH_SHELL_VIEWPORT_ORIGIN_GHIDRA_REPORT.md`
- `docs/research/skirmish-ui/SKIRMISH_SHELL_ACTIVE_RENDER_PATH_REINVESTIGATION_GHIDRA_REPORT.md`
- `docs/research/SKIRMISH_SHELL_UI_SYSTEM_MODEL_SYNTHESIS.md`
- `docs/research/traces/SKIRMISH_SHELL_CHROME_FIRST_PAINT_800X600_VISUAL_TRACE.md`
- Rust files checked: `src/app.rs`, `src/ui/main_menu.rs`, `src/ui/main_menu_shell/state.rs`, `src/ui/skirmish_shell/layout.rs`, `src/app_skirmish_shell_render.rs`, `src/render/skirmish_shell_chrome.rs`.
