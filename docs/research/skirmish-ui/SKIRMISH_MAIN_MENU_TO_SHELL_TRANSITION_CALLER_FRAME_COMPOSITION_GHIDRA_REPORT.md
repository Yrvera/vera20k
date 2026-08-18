# Skirmish Main Menu To Shell Transition Caller And Frame Composition - Ghidra Research Report

**Date:** 2026-05-27
**Address(es):** `MainMenuDialog0xE2_Proc_00531F60 @ 0x00531F60`, `FUN_00531CC0`, `Main_Game @ 0x0052D9A0`, `FUN_0060D380`, `FUN_006AE2C0`, `FUN_00622B50`, `FUN_006071E0`, `FUN_00608260`, `FUN_00608070`, `0x00612690`, `0x005E6B49`
**Investigation Mode:** coverage-map, rescoped from intended exhaustive-slice because static Ghidra resolves the caller chain and draw-call structure but not exact runtime framebuffer pixels for each transition tick.
**Claimed Scope:** verified caller chain from main menu button `0x683` toward offline Skirmish setup, whether that path has a direct main-menu-to-`0x102` transition, the live callers of the nonzero shell transition helper that were checked, and frame-composition obligations for any Rust transition implementation.
**Non-Scope:** full resource-template extraction for the intermediate Single Player shell dialog, runtime framebuffer capture, all WOL/network shell dialogs, and a complete semantic recovery of the unbounded shell/global path around `0x00612690`.
**Confidence:** High for the main-menu return-code path and `0x102` launcher. High for `FUN_00608260` and `FUN_00608070` transition mechanics. Medium for the exact semantic owner of `0x00612690` because Ghidra has no function boundary there in this database. Medium-low for exact per-pixel frame appearance without runtime capture.
**Active in YR:** Yes. The main menu `0xE2`, shell dialog loop, and offline Skirmish dialog `0x102` are active standard Yuri's Revenge paths. The `FUN_006071E0` transition helper is active in shell UI paths, but this pass does not prove a direct `0xE2` main-menu-to-`0x102` nonzero transition.

## 1. Summary

The native path is not "main menu Single Player button directly opens Skirmish setup." The verified path is:

1. Main menu dialog `0xE2` button `0x683` writes return code `1`.
2. `FUN_00531CC0` returns that code to `Main_Game`.
3. `Main_Game` case `1` calls `FUN_0060D380(1)`, a downstream shell dialog loop.
4. A later return code `0x0B` sets `g_GameMode = 5`.
5. `Main_Game` then calls `FUN_006AE2C0`, which opens and pumps offline Skirmish dialog `0x102`.

Therefore Rust's current direct `MainMenuShellAction::SinglePlayer -> native_skirmish_shell_active -> render_skirmish_shell` shortcut collapses at least one native shell layer. A direct animated side change from main menu to Skirmish can be useful for the current Rust UI, but it is not yet proven to be the native YR mechanism for `0xE2 -> 0x102`.

## 2. Existing Docs Status

The prior docs are useful but not complete for this exact question:

| Prior document | What it already proves | Gap this report resolves |
|---|---|---|
| `skirmish-ui/SKIRMISH_FUN_006071E0_SHELL_TRANSITION_REDRAW_PATH_GHIDRA_REPORT.md` | `FUN_006071E0` is a temporary transition/redraw helper; `DL=0` sends `0x4ED`, `DL=1` sends `0x4EC`; sleeps `0x1E` ms per frame. | It did not prove whether the main menu `0x683` path reaches Skirmish through this helper. |
| `FUN_006071E0_SLIDE_IN_FRAME_SCHEDULE_GHIDRA_REPORT.md` | Frame schedule and flag-byte formulas for the helper. | Older broad wording treated the helper as "every main-menu button click"; this report narrows that to checked shell-transition callers and records the unresolved `0x00612690` semantics. |
| `MAIN_MENU_VISUAL_ASSETS_GHIDRA_REPORT.md` and main-menu `0xE2` reports | Main menu resource IDs, button art, RA2TS child, and button return codes. | They do not follow return code `1` through `Main_Game` to Skirmish setup. |
| `skirmish-ui/SKIRMISH_STATIC_REVEAL_ANIMATION_0X102_GHIDRA_REPORT.md` | `0x4EC -> 0x4EE` starts kind-1 static reveal; first-paint `DL=0` does not. | It did not decide whether `0x4EC` is part of initial main-menu-to-Skirmish entry. |

## 3. Verified Caller Chain

### 3.1 Main menu `0xE2` button

`MainMenuDialog0xE2_Proc_00531F60` delegates to common shell proc `FUN_00622B50` first. If not consumed, its `WM_COMMAND 0x111` handler masks the low word and maps:

| Control | Return code |
|---:|---:|
| `0x683` Single Player | `1` |
| `0x684` Westwood Online | `2` |
| `0x578` Network | `3` |
| `0x686` Movies/Credits | `4` |
| `0x55C` Options | `5` |
| `0x3EE` Exit | `6` |

Evidence: `MainMenuDialog0xE2_Proc_00531F60` decompile.

### 3.2 Main menu loop

`FUN_00531CC0` creates the main menu dialog, stores a local result pointer via `SetWindowLongA(hWnd, 8, &local_1c)`, initializes RA2TS child `0x71A`, pumps messages while `local_1c == 0x12`, destroys the dialog via `FUN_00622720`, then returns `local_1c`.

Evidence: `FUN_00531CC0` decompile.

### 3.3 Main game dispatch

`Main_Game` receives return code `1` and calls `FUN_0060D380(1)`. It does not directly call `FUN_006AE2C0` in case `1`.

`Main_Game` reaches offline Skirmish only when a later shell return code `0x0B` sets `g_GameMode = 5`; in the setup switch, case `5` calls `FUN_006AE2C0()`.

Evidence: `Main_Game @ 0x0052D9A0` decompile.

### 3.4 Downstream shell dialog loop

`FUN_0060D380(1)` creates a shell dialog via `FUN_00622650`, shows it, calls `FUN_0054F720`, calls `FUN_0052B9B0` when the parameter is nonzero, and pumps until its local result is nonzero. This is an intermediate shell layer between main menu return code `1` and Skirmish setup entry.

Evidence: `FUN_0060D380` decompile.

### 3.5 Offline Skirmish setup launcher

`FUN_006AE2C0` initializes country/house data, calls `FUN_0072CF40`, creates a shell dialog via `FUN_00622650`, stores the HWND in `DAT_00B0B59C`, calls `FUN_00622800`, and pumps until local result is `0x617` Start or `0x5C0` Back. It returns true only for `0x617`.

No direct call to `FUN_00608260`, `FUN_00608070`, or `FUN_006071E0` appears in the `FUN_006AE2C0` decompile.

Evidence: `FUN_006AE2C0` decompile.

## 4. Transition Helpers

### 4.1 Direct nonzero transition: `FUN_00608260`

`FUN_00608260(HWND)` is the direct transition helper. It gates on:

- `FUN_0069BBE0() == 0`
- shell record hash active: `DAT_00AC1B04 != 0`
- record byte `+0xC1 != 0`
- record integer `piVar1[0x2D] == 1`, which is byte offset `+0xB4`
- `IsWindowVisible(hwnd) != 0`

On success it plays a sound, saves enabled state, disables the parent, enumerates child windows through `LAB_00606800` with `1`, calls `FUN_006071E0` with `DL=1`, enumerates children with `0`, restores enabled state, invalidates the parent, and returns `1`.

Assembly confirms the mode:

- `0x0060833F`: `MOV DL, 0x1`
- `0x00608341`: `MOV ECX, ESI`
- `0x00608343`: `CALL 0x006071E0`

Evidence: `FUN_00608260` decompile; assembly context at `0x0060833F..0x00608343`.

### 4.2 Deferred paint transition: `FUN_00608070` and common paint

`FUN_00608070(HWND)` uses the same visible/shell-mode gates, plays the sound, disables the parent/children, sets record byte `+0xC2 = 1`, invalidates the parent, then pumps until the byte clears or 5000 ms elapse.

Common shell paint in `FUN_00622B50` consumes the equivalent deferred dirty byte:

- If the dirty byte is set during `WM_PAINT`, it sends `0x4E2` to child `0x71A` when present.
- It calls `FUN_006071E0` with `DL=0`.
- It clears the dirty byte and validates the parent.

Assembly confirms zero mode:

- `0x00622CA6`: `XOR DL, DL`
- `0x00622CA8`: `MOV ECX, ESI`
- `0x00622CAA`: `CALL 0x006071E0`

In `DL=0`, `FUN_006071E0` sends `0x4ED`, not `0x4EC`, so it does not start the kind-1 static reveal path.

Evidence: `FUN_00608070`, `FUN_00622B50`; assembly context at `0x00622CA6..0x00622CAA`; prior static-reveal report.

## 5. Checked Callers

| Caller | Verified behavior | Main-menu-to-`0x102` relevance |
|---|---|---|
| `0x005E6B49 -> FUN_00608260` | Occurs in the Choose Map modal callback path. Context calls `FUN_005E7BF0`, `FUN_005E7160`, then `FUN_0052FEC0`; if not diverted, calls `FUN_00608260` and shows the parent with `ShowWindow(hwnd, 5)`. | Not initial main-menu-to-Skirmish entry. This is after `0x102` is already active and the Choose Map flow returns. |
| `0x00612690 -> FUN_00608260` | Shell/global path with state word `EDI + 0x1FC`, helper call with argument `4`, then `FUN_00608260`; success writes state `3`. | Potentially important for generic shell button transitions, but semantic owner and exact message path were not recovered because no containing function boundary exists in this Ghidra database. Treat as unresolved before claiming exact main-menu button click parity. |
| `FUN_00622B50 WM_PAINT -> FUN_006071E0` | Deferred dirty-paint path passes `DL=0`, sends `0x4ED`, and does not start text reveal. | Active shell paint path, but not the direct smooth side-change/reveal path. |
| `FUN_006ACEE0 Choose Map 0x5AA -> FUN_00608070 -> ShowWindow(0) -> FUN_005E68A0` | Schedules deferred transition/redraw before hiding `0x102` for Choose Map. | Post-entry Skirmish workflow, not initial main-menu-to-Skirmish entry. |

## 6. Frame Composition Ledger For `FUN_006071E0`

This ledger is the implementation obligation if Rust models the native transition helper. It should not be treated as proof that the helper is the direct `0xE2 -> 0x102` entry animation.

| Stage | Composition behavior | Evidence |
|---|---|---|
| Gate and setup | Caller must pass parent HWND in `ECX` and mode in `DL`. Direct helper uses `DL=1`; common paint uses `DL=0`. | Assembly contexts `0x0060833F..0x00608343`, `0x00622CA6..0x00622CAA`. |
| Child inventory | Enumerates visible/eligible children through `FUN_0060A180` and `FUN_0060A250`, building counts used for the schedule. | Prior `SKIRMISH_FUN_006071E0...` report; `FUN_006071E0` decompile. |
| Schedule | Allocates `(count + 3) * 4`, fills ascending per-child entries, computes max schedule entry, then runs `max + 6` ticks. | Prior `FUN_006071E0_SLIDE_IN_FRAME_SCHEDULE...` report. |
| Per-tick draw | Draws shell shapes via `CC_Draw_Shape` with draw flags including `0x400` and argument `1000`; uses shell globals such as `g_SDTP_SHP`, `g_SDBTNANM_SHP`, radar background/frame globals, and shell rect globals. | `FUN_006071E0` decompile and prior reports. |
| Optional groups | Data bytes around `+0xD5..+0xD7` gate extra right-panel/radar/button groups. Defaults and writers are not fully inventoried in this pass. | Prior transition report; open question retained. |
| Cadence | Flushes display surfaces/display chain work and sleeps `Sleep(0x1E)` per tick. | `FUN_006071E0` decompile. |
| Completion, `DL=0` | Sends parent message `0x4ED`; no static reveal start. | Prior transition/static-reveal reports. |
| Completion, `DL=1` | Uses Rules audio field `+0x750` (`ShellButtonSlideSound`), drains display chain, sends parent message `0x4EC`; `FUN_00622B50` broadcasts to children and kind-1 statics receive `0x4EE`. | Prior transition/static-reveal reports; assembly mode evidence. |

Stock INI has `ShellButtonSlideSound=` empty in both `rules.ini` and `rulesmd.ini`; the call is still real, but retail stock resolves to no audible slide sound unless overridden by rules.

## 7. Current Rust Delta

Current Rust maps the main-menu Single Player button directly into the Skirmish shell:

- `src/ui/main_menu_shell/state.rs:46` maps `SinglePlayer0x683` to `MainMenuShellAction::SinglePlayer`.
- `src/ui/main_menu_shell/state.rs:59` preserves native return code `1`.
- `src/app.rs:1474..1477` sets `main_menu_show_native_skirmish_shell = true`, enters shell window mode, and ensures Skirmish shell chrome immediately.
- `src/app.rs:2083..2088` renders `app_skirmish_shell_render::render_skirmish_shell` whenever `native_skirmish_shell_active(state)` is true.

That matches the desired local shortcut, but not the verified native route through `FUN_0060D380(1)` and later return code `0x0B`. If implementing the requested smoothing on the current shortcut, label it as a Rust UI bridge until the intermediate Single Player shell is implemented.

## 8. Implementation Handoff

If the goal is pragmatic smoothness for the current Rust shortcut:

1. Add an app/shell-level transition state above `render_main_menu_shell` and `render_skirmish_shell`.
2. On `MainMenuShellAction::SinglePlayer`, do not flip `main_menu_show_native_skirmish_shell` instantly. Start a blocking/effectively modal transition state that owns elapsed time/tick index.
3. Compose frames from the current main-menu shell and target Skirmish shell surfaces in the same shell/app rendering layer; do not move this into `sim/`.
4. Use the verified cadence as the first parity anchor: `30 ms` tick steps and a `max_schedule + 6` completion rule if child scheduling is modeled.
5. After the final tick, set `main_menu_show_native_skirmish_shell = true`, trigger the Skirmish shell reveal start equivalent only if modeling the `DL=1 -> 0x4EC -> 0x4EE` path, and invalidate/repaint once.
6. Keep this flagged as DRIFT from exact native shell flow until `FUN_0060D380` and its intermediate shell return `0x0B` path exist in Rust.

If the goal is strict native parity:

1. Implement the missing intermediate Single Player shell dialog first.
2. Verify whether its button/subclass path reaches `0x00612690 -> FUN_00608260` on the Skirmish selection control.
3. Only then wire `FUN_006071E0`-equivalent animation to the exact native control/action that returns `0x0B`.

## 9. Open Question Log

- [RESOLVED] OQ-01 - Do prior docs already cover `FUN_006071E0`? Yes; transition mechanics and static reveal are covered, but not this caller chain.
- [RESOLVED] OQ-02 - Does main menu button `0x683` directly call Skirmish `0x102`? No; it writes return code `1`.
- [RESOLVED] OQ-03 - What consumes return code `1`? `Main_Game` case `1`, which calls `FUN_0060D380(1)`.
- [RESOLVED] OQ-04 - What opens offline Skirmish `0x102`? `Main_Game` later handles return code `0x0B`, sets `g_GameMode = 5`, and calls `FUN_006AE2C0`.
- [RESOLVED] OQ-05 - Does `FUN_006AE2C0` directly call the transition helper on entry? No direct call appears in its decompile.
- [RESOLVED] OQ-06 - What is the nonzero transition helper? `FUN_00608260`, which calls `FUN_006071E0` with `DL=1`.
- [RESOLVED] OQ-07 - What does zero-mode paint do? Common paint calls `FUN_006071E0` with `DL=0`, then sends `0x4ED`, not `0x4EC`.
- [RESOLVED] OQ-08 - Is `0x005E6B49` initial Skirmish entry? No; it is in the Choose Map modal callback/return path.
- [DEFERRED] OQ-09 - What exact high-level function owns `0x00612690`? Needs function-boundary recovery or debugger trace; current Ghidra database has no containing function.
- [DEFERRED] OQ-10 - Does the intermediate Single Player shell's Skirmish-selection control reach `0x00612690 -> FUN_00608260`? Needs the intermediate dialog resource/control and a live trace or better static boundary.
- [DEFERRED] OQ-11 - Exact per-pixel transition frames? Needs runtime capture or a fully instrumented renderer comparison.
- [DEFERRED] OQ-12 - Complete writer inventory for `+0xD5..+0xD7` optional transition group flags? Prior docs left this incomplete; still not needed to answer direct main-menu-to-`0x102` path.
- [RESOLVED] OQ-13 - Is stock slide sound audible? Stock `ShellButtonSlideSound` key is empty in `rules.ini` and `rulesmd.ini`; the call is real but stock rules resolve to no audible slide sound.
- [RESOLVED] OQ-14 - Is current Rust exact native flow? No; current Rust jumps from Single Player directly to native Skirmish shell rendering.
- [RESOLVED] OQ-15 - Is this TS legacy only? No; all checked functions are active YR shell/skirmish paths, though exact `0x00612690` semantics remain unresolved.

## 10. Evidence Log

- Ghidra decompiled/read-only: `MainMenuDialog0xE2_Proc_00531F60`, `FUN_00531CC0`, `Main_Game @ 0x0052D9A0`, `FUN_0060D380`, `FUN_006AE2C0`, `FUN_006ACEE0`, `FUN_00622B50`, `FUN_00608260`, `FUN_00608070`, `FUN_005E68A0`.
- Ghidra assembly contexts: `0x0060833F`, `0x00608343`, `0x00622CA6`, `0x00612690`, `0x005E6B49`, `0x006AD947`.
- Source scan: `src/app.rs`, `src/ui/main_menu_shell/state.rs`.
- INI scan: `ini/rules.ini`, `ini/rulesmd.ini` for `ShellButtonSlideSound`, `GUIMainButtonSound`, and `GenericClick`.

