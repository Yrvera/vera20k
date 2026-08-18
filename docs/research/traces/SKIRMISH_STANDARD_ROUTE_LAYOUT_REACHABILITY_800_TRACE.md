# Skirmish Standard Route Layout Reachability 800 Trace

Scenario: from the normal Rust main-menu path at `800x600`, choose Skirmish/Single Player and compare whether the player-visible route reaches the retail-style pixel Skirmish shell/dialog `0x102` coordinate basis, versus standard offline Yuri's Revenge reaching dialog `0x102` from the native offline Skirmish launcher. Scope is layout/pixel-position reachability only; gameplay launch is out of scope.

## Verdict

PASS: 2 | FAIL: 2 | UNCHECKED: 1 | NOT-IMPLEMENTED: 1

Current Rust now reaches the pixel Skirmish shell from the normal main-menu route, but not by the active gamemd route. The player-visible endpoint after the Rust bridge is the native-style shell renderer, and the checked `800x600` coordinate basis/key rects match the documented `0x102` values. The route to that endpoint is still DRIFT: Rust uses a temporary `main-menu -> Skirmish` bridge transition and skips the verified intermediate YR shell path that consumes main-menu return code `1` and later returns `0x0B` before `FUN_006AE2C0` creates dialog `0x102`.

## Pipeline

gamemd active YR route:

`Main menu dialog 0xE2 button 0x683 -> return code 1 -> Main_Game -> FUN_0060D380(1) intermediate shell loop -> later return code 0x0B -> g_GameMode=5 -> FUN_006AE2C0 -> FUN_00622650(dialog 0x102/proc 0x006AE3F0) -> ShowWindow -> FUN_0060C4A0 fullscreen host/child resize -> player sees 0x102`

Current Rust route:

`main menu shell mouse_up -> MainMenuShellAction::SinglePlayer -> start_main_menu_to_skirmish -> 14-frame ShellBridgeTransition at 30 ms/frame -> main_menu_show_native_skirmish_shell=true -> render_skirmish_shell -> compute_fixed_800_layout(800,600) -> player sees pixel shell`

## Stage Table

| Stage | Boundary Output | gamemd 800x600 | Rust 800x600 | Verdict |
|---:|---|---|---|---|
| 1 | Main-menu control identity / return intent | `0x683` maps to return code `1`. | `SinglePlayer0x683` maps to `MainMenuShellAction::SinglePlayer`; `return_code_for_action(SinglePlayer)` is `Some(1)`. | PASS |
| 2 | Native shell route between main menu and Skirmish setup | `Main_Game` case `1` calls `FUN_0060D380(1)`; only a later return code `0x0B` sets `g_GameMode=5` and reaches `FUN_006AE2C0`. | No intermediate Single Player shell route is implemented for this path; handler starts the bridge shortcut directly. | NOT-IMPLEMENTED |
| 3 | Transition/timing before `0x102` endpoint | `FUN_006AE2C0` opens/pumps `0x102`; prior caller-chain report found no direct `FUN_006071E0` entry call in `FUN_006AE2C0`. | `ShellBridgeTransition` runs `14 * 30 ms = 420 ms` before setting `main_menu_show_native_skirmish_shell=true`. | FAIL |
| 4 | Final `0x102` host coordinate basis | Parent HWND is moved to `(0,0,800,600)`; child resize policy uses the active `0x102` matrix, not scaling. | `compute_fixed_800_layout(800,600)` has centered offset `(0,0)` and screen `(0,0,800,600)`. | PASS |
| 5 | Full framebuffer equality for the route endpoint | Not captured in this slot. | Not captured in this slot. | UNCHECKED |
| 6 | Route mechanism equivalence | Active route includes native return-code shell loop and standard launcher-created dialog `0x102`. | Rust route is explicitly documented as bridge/DRIFT code and bypasses the verified intermediate shell mechanism. | FAIL |

## Checked Numeric Layout

At the reached Rust endpoint, the checked key `0x102` rectangles match gamemd's verified `800x600` matrix:

| Control | gamemd final rect | Rust final rect | Verdict |
|---|---:|---:|---|
| Start `0x617` | `(644,241,156,42)` | `(644,241,156,42)` | PASS |
| Choose Map `0x5AA` | `(644,283,156,42)` | `(644,283,156,42)` | PASS |
| Map preview `0x468` | `(644,37,144,112)` | `(644,37,144,112)` | PASS |
| Back `0x5C0` | `(644,535,156,42)` | `(644,535,156,42)` | PASS |

These are endpoint-layout checks only. They do not prove the whole framebuffer, every child, text rasterization, or transition frames are pixel-identical.

## Failures And Missing Pieces

### Stage 2 - Intermediate native shell route is not implemented

Player-visible difference: the Rust player does not traverse the same shell path as standard YR before reaching the Skirmish setup. At the moment, Single Player acts as a shortcut into the Skirmish pixel shell.

Rust evidence: `src/app.rs:1494` handles `MainMenuShellAction::SinglePlayer`; `src/app.rs:1499` starts `start_main_menu_to_skirmish`; `src/app_shell_transition.rs:88` clears both setup flags and stores a `ShellBridgeTransition`; `src/app_shell_transition.rs:191` sets `main_menu_show_native_skirmish_shell = true` at completion.

gamemd evidence: `SKIRMISH_MAIN_MENU_TO_SHELL_TRANSITION_CALLER_FRAME_COMPOSITION_GHIDRA_REPORT.md` verifies `0x683 -> 1`, `Main_Game` case `1 -> FUN_0060D380(1)`, and only a later `0x0B` route sets `g_GameMode = 5` before calling `FUN_006AE2C0`. The report marks these paths Active in YR: Yes, not TS legacy.

### Stage 3 - Rust inserts a bridge transition before the endpoint shell

Player-visible difference: the endpoint is delayed/animated by a Rust-only bridge before the shell becomes active. This can move pixels during the route even though the final checked shell rects match.

Rust evidence: `src/app_shell_transition.rs:14` sets `SHELL_BRIDGE_FRAME_MS = 30`; `src/app_shell_transition.rs:15` sets `SHELL_BRIDGE_FRAME_COUNT = 14`; `src/app_shell_transition.rs:65` advances one frame per duration step; `src/render/shell_transition_pass.rs:1` documents the pass as a temporary bridge, not a verified native shell transition.

gamemd evidence: the caller-frame-composition report records that `FUN_006AE2C0` directly opens and pumps dialog `0x102` and that no direct `FUN_006071E0` entry call appears in that launcher. Its open-question log resolves that current Rust is not the exact native flow and that the checked functions are active YR shell/skirmish paths.

### Stage 6 - Route mechanism differs even though endpoint layout is reached

Player-visible difference: the player reaches the pixel shell, but the path there is not the standard YR path. If the intermediate Single Player shell has visible frames, focus, sounds, or status text before selecting Skirmish, this shortcut cannot be pixel-parity.

Rust evidence: `src/app_shell_transition.rs:1` calls this an app-level bridge for the temporary shortcut; `src/app.rs:2120` renders the bridge first, then `src/app.rs:2125` renders `render_skirmish_shell` only after native shell activation.

gamemd evidence: `SKIRMISH_MAIN_MENU_TO_SHELL_TRANSITION_CALLER_FRAME_COMPOSITION_GHIDRA_REPORT.md` sections 3.1-3.4 and 9 resolve that the main-menu button does not directly call Skirmish `0x102`; it returns `1`, then the later standard route reaches `FUN_006AE2C0`.

## Unchecked

Full-frame pixel equality is unchecked. This slot did not launch retail YR, capture a gamemd framebuffer, launch Rust, or diff screenshots. Per trace-action rules, no aggregate visual PASS is claimed without both framebuffers.

Exact native timing from the initial main-menu click through the intermediate shell to the later `0x0B` Skirmish selection is also not fully computed in this slot. The failure above is based on the verified mechanism difference and the computed Rust bridge duration.

## Adjacent Findings

Complete `0x102` child inventory, Choose Map `0x6B`, right-panel/button pressed offsets, preview marker layout, combo/dropdown geometry, and gameplay launch are adjacent swarm slots or separate traces. This report only traces standard route/layout reachability at `800x600`.

## Sources

- `docs/research/skirmish-ui/SKIRMISH_MAIN_MENU_TO_SHELL_TRANSITION_CALLER_FRAME_COMPOSITION_GHIDRA_REPORT.md`
- `docs/research/skirmish-ui/SKIRMISH_HIGH_RES_SHELL_HOSTING_ORIGIN_GHIDRA_REPORT.md`
- `docs/research/skirmish-ui/SKIRMISH_RESIZE_SHELL_CHILD_CONTROL_0060C0C0_COMPLETE_0X102_POLICY_GHIDRA_REPORT.md`
- `docs/research/skirmish-ui/SKIRMISH_0X102_COMPLETE_CHILD_RECT_MATRIX_GHIDRA_REPORT.md`
- `docs/research/SKIRMISH_UI_CURRENT_SYSTEM_MODEL_SYNTHESIS.md`
- Current Rust read-only scan: `src/app.rs`, `src/app_shell_transition.rs`, `src/render/shell_transition_pass.rs`, `src/ui/main_menu_shell/state.rs`, `src/ui/skirmish_shell/layout.rs`, `src/app_skirmish_shell_render.rs`.
