# Shell Menu Transition System Model Synthesis

**Date:** 2026-05-27  
**Output type:** conflict-map with implementation-safe route facts  
**System:** Main menu / Single Player shell / Skirmish shell transition path  
**Included surfaces:** main menu Single Player result, dialog `0x100`, control `0x579`, result `0x0B`, Skirmish dialog `0x102`, generic shell transition helper `FUN_00608260 -> FUN_006071E0`, `0x00612690` child subclass caller, right-panel SHP assets and stock sounds.  
**Non-scope:** runtime debugger capture, exact framebuffer replay of an unproven route-active transition, Rust implementation.

## 1. Current Model

The current verified native route is:

`main menu Single Player 0x683 -> result 1 -> Main_Game -> FUN_0060D380(1) -> dialog 0x100 -> Skirmish button 0x579 -> result 0x0B -> g_GameMode=5 -> offline Skirmish dialog 0x102`.

The route result is implementation-safe: dialog proc `0x0052D640` handles parent `WM_COMMAND`, masks `LOWORD(wParam)`, matches `0x579`, and writes result `0x0B` directly. The result-write proc contains no direct call to `FUN_00608260` or `FUN_006071E0`.

The attractive shell transition machinery is real but not route-proven for this specific click. `FUN_00608260` calls `FUN_006071E0` with `DL=1`, and `FUN_006071E0` is the generic 30 ms shell redraw/transition helper that consumes `SDBTNANM`, `SDMPBTN`, `SDWRNTMP`, `SDTP`, and related chrome assets according to shell-record flags. However, for the standard Single Player `0x100` Skirmish command, current static evidence only proves eligibility/plausibility, not actual runtime reachability.

The best current explanation for `0x00612690` is: it is inside the unlabelled shell child subclass dispatcher `0x00610CA0..0x006128FE`, installed by `FUN_0060F9A0` via `SetWindowLongA(hwnd, GWL_WNDPROC=-4, 0x00610CA0)`. It may call `FUN_00608260` only on a paint/state path when the per-control record `+0x1FC == 1`; it writes `2` before the call and `3` after success. It is not the direct `0x579 -> 0x0B` command-result owner.

## 2. Claim Table

| Claim | Best evidence | Status | Confidence | Active in YR | Safe? |
|---|---|---|---|---|---|
| Main menu Single Player enters dialog `0x100`, not Skirmish setup directly. | `0x0052DD39..0x0052DD4B`; `SKIRMISH_FUN_0060D380_SINGLE_PLAYER_0X100_TO_0X0B_GHIDRA_REPORT.md`; `SINGLE_PLAYER_0X100_SKIRMISH_0X579_ROUTE_GHIDRA_REPORT.md` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| Dialog `0x100` has visible Skirmish button `0x579`, style `0x5000000B`. | retail `RT_DIALOG` extraction in `SINGLE_PLAYER_0X100_SKIRMISH_0X579_ROUTE_GHIDRA_REPORT.md` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| `0x579` writes result `0x0B` directly in `0x0052D640`. | raw PE disassembly `0x0052D6F1..0x0052D720` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| `0x0052D640` directly calls `FUN_00608260` / `FUN_006071E0`. | raw PE disassembly `0x0052D640..0x0052D785` | contradicted | high | no | DOC_PATCH_READY |
| Generic `FUN_00608260 -> FUN_006071E0` shell transition helper exists. | `FUN_006071E0_SLIDE_IN_FRAME_SCHEDULE_GHIDRA_REPORT.md`; `SHELL_DIALOG_SLIDE_TRANSITION_TRANSFORM_GHIDRA_REPORT.md`; swarm slot 3 | confirmed | high | conditional | IMPLEMENTATION_SAFE only for generic helper, not this route |
| Standard `0x579` click reaches `0x00612690 -> FUN_00608260`. | subclass plausibility plus unresolved live state gate | unknown | medium | conditional/unchecked | NEEDS_REINVESTIGATE |
| Source dialog `0x100` enables optional SDMPBTN/SDWRNTMP transition groups. | `FUN_0060CAF0`, `FUN_0060C930`, `FUN_0060CCC0`, `FUN_0060CDB0` in swarm slot 3 | contradicted | high | no | DOC_PATCH_READY |
| Destination dialog `0x102` enables `+0xD9/+0xDA` steady-state chrome groups. | same classifiers in swarm slot 3; `SKIRMISH_0X102_TOP_PREVIEW_CHROME_SDTP_SDMPBTN_GHIDRA_REPORT.md` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| Stock `ShellButtonSlideSound` is audible for this route. | `ini/rules.ini`, `ini/rulesmd.ini`, helper sound site | contradicted for stock data | high | stock silent | DOC_PATCH_READY |
| Current Rust has the `0x100` shell and `0x579 -> 0x0B` identity. | `src/ui/single_player_shell/state.rs`; `src/app.rs` scan | confirmed | high | n/a | IMPLEMENTATION_SAFE |
| Current Rust whole-screen bridge is native `FUN_006071E0` parity. | Rust bridge vs binary helper reports | contradicted | high | n/a | DOC_PATCH_READY |

## 3. Implementation-Safe Facts

- Preserve the intermediate shell. Main-menu Single Player must open dialog-equivalent `0x100`; it must not shortcut to Skirmish setup.
- Preserve `Skirmish0x579 -> Some(0x0B)` as the route boundary even if a visual transition is added around it.
- Preserve separate click/pressed-art semantics. Ordinary button feedback uses the owner-draw button path and stock `GUIMainButtonSound=MenuClick` / `GenericClick=MenuClick`; `ShellButtonSlideSound=` is empty in stock `rules.ini` and `rulesmd.ini`.
- For native dialog metadata, `0x100` clears optional `+0xD9/+0xDA/+0xDB/+0xDC` groups, while `0x102` sets `+0xD9/+0xDA` and clears `+0xDB/+0xDC`.
- Any visual bridge from Single Player shell to Skirmish shell may be useful UX, but it must remain labeled bridge/DRIFT until runtime proof shows the native click reaches the transition caller.

## 4. Doc-Patch-Ready Facts

- Replace "main menu Single Player directly transitions to Skirmish" with the verified `0x683 -> 1 -> 0x100 -> 0x579 -> 0x0B -> 0x102` route.
- Replace "every shell button click triggers `FUN_00608260 -> FUN_006071E0`" with "the helper is live generic shell machinery, but the `0x100` Skirmish command writes `0x0B` directly; route-active helper reachability is unproven."
- Replace "`0x00612690` is inside `OwnerDraw_Button_00612B70`" with "`0x00612690` is inside the shell child subclass dispatcher `0x00610CA0..0x006128FE`; `OwnerDraw_Button_00612B70` is a selected callback under that dispatcher."
- Replace "Rust lacks dialog `0x100`" with "Rust now has `src/ui/single_player_shell` with control ids and return codes; remaining parity work is transition/reveal ownership, sound/paint timing, and pixel/layout verification."

## 5. Stale Or Superseded Claims

Older wording that treats the current Rust bridge as evidence for native main-menu-to-Skirmish parity is stale. The bridge is currently a whole-screen compositor and not a proven native `FUN_006071E0` implementation.

Older wording that implies `SDBTNANM` / `SDMPBTN` / `SDWRNTMP` are all route-active for `0x100 -> 0x102` is stale. Those assets are verified generic-helper or destination-chrome assets, but source `0x100` clears the optional groups and the route-active helper call remains unproven.

Older wording that treats `ShellButtonSlideSound` as the player-visible click cue is stale for stock YR. The key exists, but stock value is empty; ordinary click feedback comes from the owner-draw button sound paths.

## 6. Cross-Doc Conflicts

The main remaining conflict is wording, not the verified route: several older transition docs correctly identify real helper machinery but over-attach it to the standard main-menu/Single Player Skirmish path. Newer swarm reports narrow that claim: helper machinery is real; route-active helper reachability for `0x579` is still unchecked.

This synthesis should be treated as the current canonical interpretation for implementation planning until a runtime trace proves or refutes `0x00612690` activation during the retail `0x579` click.

## 7. Needs Re-Investigation

1. Runtime trace `FUN_00608260` and `0x00612690` while clicking retail Single Player `0x100` Skirmish `0x579`.
   - Goal: prove whether `record+0x1FC == 1` occurs before the direct `0x0B` route completes.
   - Suggested command: `/re-investigate runtime Single Player 0x100 Skirmish 0x579 hits 0x00612690 FUN_00608260`

2. Dataflow into shell per-control record `+0x1FC`.
   - Goal: identify which message/state branch writes `1` for candidate controls, and whether `0x579` receives that state in standard YR.
   - Suggested command: `/re-investigate shell child dispatcher 0x00610CA0 record 0x1FC writers`

3. Framebuffer capture only after route-active caller proof.
   - Goal: produce exact per-frame pixel/asset schedule for the target transition if the native route really uses it.
   - Suggested command: `/trace-action retail Single Player shell Skirmish transition frame capture`

## 8. Do-Not-Implement Notes

- Do not reintroduce a direct main-menu-to-Skirmish transition as parity.
- Do not make `FUN_00608260` required for route correctness; `0x579 -> 0x0B` is direct.
- Do not draw `0x102` steady-state chrome flags during source `0x100` unless native per-dialog flags justify it.
- Do not claim the current whole-screen Rust bridge is native `FUN_006071E0` parity.
- Do not use `ShellButtonSlideSound` as a stock audible click or slide cue for this path.

## 9. Current Rust Handoff

Current Rust already has the implementation-safe route identity:

- `src/app.rs`: main-menu Single Player opens the Single Player shell; `SinglePlayerShellAction::Skirmish` enters native Skirmish shell.
- `src/ui/single_player_shell/state.rs`: `Skirmish0x579` maps to `SinglePlayerShellAction::Skirmish`, and its route code is `Some(0x0B)`.
- `src/app_shell_transition.rs`: existing transition code is a bridge/DRIFT whole-screen compositor and should either remain quarantined or be renamed/reworked before being used for this path.

Recommended near-term implementation policy: if the visual transition is implemented now, make it a clearly labeled Rust bridge from Single Player shell render target to Skirmish shell render target. Keep route identity and native result ordering intact. Do not mark it parity until the runtime trace closes `0x00612690` reachability.

## 10. Source Ledger

- `docs/research/SINGLE_PLAYER_0X100_SKIRMISH_0X579_ROUTE_GHIDRA_REPORT.md`
- `docs/research/SINGLE_PLAYER_TO_SKIRMISH_FUN_006071E0_FLAGS_ASSETS_GHIDRA_REPORT.md`
- `docs/research/SHELL_TRANSITION_CALLER_00612690_OWNER_GHIDRA_REPORT.md`
- `docs/research/MAIN_MENU_SHELL_TRANSITION_ASSET_SURVEY_2026_05_27.md`
- `docs/research/FUN_006071E0_SLIDE_IN_FRAME_SCHEDULE_GHIDRA_REPORT.md`
- `docs/research/SHELL_DIALOG_SLIDE_TRANSITION_TRANSFORM_GHIDRA_REPORT.md`
- `docs/research/SHELL_BUTTON_SLIDE_SOUND_CALL_SITE_GHIDRA_REPORT.md`
- `docs/research/skirmish-ui/SKIRMISH_FUN_0060D380_SINGLE_PLAYER_0X100_TO_0X0B_GHIDRA_REPORT.md`
- `docs/research/skirmish-ui/SKIRMISH_NATIVE_SINGLE_PLAYER_ROUTE_TO_0X102_RECHECK_GHIDRA_REPORT.md`
- `docs/research/skirmish-ui/SKIRMISH_FUN_006071E0_SHELL_TRANSITION_REDRAW_PATH_GHIDRA_REPORT.md`
- `docs/research/skirmish-ui/SKIRMISH_0X102_TOP_PREVIEW_CHROME_SDTP_SDMPBTN_GHIDRA_REPORT.md`
- INI defaults: `ini/rules.ini` lines for `GUIMainButtonSound`, `GenericClick`, `ShellButtonSlideSound`; `ini/rulesmd.ini` same keys.
- Rust scan: `src/app.rs`, `src/app_shell_transition.rs`, `src/ui/single_player_shell/state.rs`.

## 11. Classification

Route model: **IMPLEMENTATION_SAFE**.  
Native transition trigger for `0x100` Skirmish click: **NEEDS_REINVESTIGATE / UNSAFE_FOR_PARITY_IMPLEMENTATION**.  
Doc correction model: **DOC_PATCH_READY** for replacing older over-broad transition claims.
