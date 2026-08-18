# Skirmish Choose Map Action Trace

**Scenario:** Default Skirmish setup dialog `0x102`; click `Choose Map` once.

**Scope:** One action only: the immediate result of pressing/releasing the `0x5AA` Choose Map button and the Rust behavior it currently maps to.

**Verdict tally:** PASS: 1 | FAIL: 2 | UNCHECKED: 4 | NOT-IMPLEMENTED: 1

## Summary

`gamemd.exe` treats `Choose Map` as a modal map-selection transition. The live offline Skirmish command handler receives control id `0x5AA`, hides the Skirmish setup dialog, calls the map-selection routine, then shows the setup dialog again and rebuilds selected map, selected mode/category, and preview state after the chooser returns.

Rust currently treats `Choose Map` as an in-place "next map" command. On mouse release over the same owner-draw button, `SkirmishShellAction::ChooseMap` increments `SkirmishShellState.selected_map_idx = (selected_map_idx + 1) % maps.len()` and returns `None`. No modal chooser opens, and the current skirmish shell renderer does not consume `selected_map_idx` to draw a real preview surface or map label.

Player-visible result: after one click, gamemd leaves the setup screen for a Choose Map dialog; Rust stays on the setup screen and silently advances the selected map used later by Start Game.

## Active-YR Evidence

- `MAIN_GAME_STATE_MACHINE_CASES_GHIDRA_REPORT.md` marks g_GameMode `5` -> `FUN_006AE2C0` as active in YR, with `0x617`/`0x5C0` return codes.
- Fresh read-only Ghidra xref: `FUN_006AE2C0` has an unconditional caller from `Main_Game` at `0x0052E168`.
- Fresh read-only Ghidra decompile of `FUN_006AE2C0` confirms it creates/pumps dialog `0x102`, stores the dialog result pointer, and loops until local result is `0x617` or `0x5C0`.
- Fresh read-only Ghidra decompile of `FUN_006AE3F0` confirms `WM_COMMAND (0x111)` calls `FUN_006ACEE0` with the control id and command notification.
- Fresh read-only Ghidra decompile of `FUN_006ACEE0` confirms a live `param_2 == 0x5AA` branch that hides the skirmish dialog, calls the map-selection routine, shows the dialog again, rebuilds map/preview-related state, and invalidates the dialog.

No TS-only or fog-of-war gates were found on this UI path.

## Pipeline

1. **Button geometry and identity**
   - gamemd: dialog resource `0x102`, `Choose Map` control id `0x5AA`, DLU rect `(425,176,108,23)`, final verified 800x600 rect `(635,286,162,37)` after right-panel anchoring.
   - Rust: `compute_layout(800,600)` gives `choose_map_button = RectPx::new(635,286,162,37)`.
   - Verdict: PASS for control identity and 800x600 hit rect.

2. **Click dispatch**
   - gamemd: standard owner-draw button behavior sends parent `WM_COMMAND`; `FUN_006AE3F0` dispatches command id `0x5AA` into `FUN_006ACEE0`.
   - Rust: mouse down stores `OwnerDrawButton::ChooseMap0x5aa`; mouse up only acts if release hits the same button; then maps it to `SkirmishShellAction::ChooseMap`.
   - Verdict: UNCHECKED. Both are release-driven, but exact Windows notification timing and command high-word values were not numerically matched against Rust event timing.

3. **Immediate action**
   - gamemd: `0x5AA` branch copies current map token, calls setup helpers, hides the setup HWND with `ShowWindow(param_1,0)`, invokes the map chooser (`FUN_005E68A0`), then later restores the setup dialog with `ShowWindow(param_1,5)`.
   - Rust: `apply_action(..., ChooseMap, maps)` increments `selected_map_idx` modulo `maps.len()` and returns `SkirmishShellAction::None`.
   - Verdict: FAIL. gamemd opens a chooser; Rust cycles the map index in-place.

4. **Dialog-loop result**
   - gamemd: `FUN_006AE2C0` continues pumping the Skirmish dialog until result `0x617` Start or `0x5C0` Back. `0x5AA` is not a loop-exit return code.
   - Rust: `handle_skirmish_shell_action` receives `None` after applying Choose Map, so it neither starts nor exits; the screen remains `MainMenu`.
   - Verdict: UNCHECKED. The high-level stay-in-setup result after chooser return is similar, but the modal subdialog interval was not implemented and no exact return-code equality exists to compare.

5. **Map selection state**
   - gamemd: selected map and selected mode/category globals are preserved or updated only after the chooser returns; the handler restores old selection globals on at least one chooser return branch.
   - Rust: default `selected_map_idx = 0`; with the local retail map directory containing 41 map files by direct extension scan, one click computes `(0 + 1) % 41 = 1`.
   - Verdict: FAIL. Rust changes selected map on the click itself; gamemd changes selection through the map chooser flow, not by one-click cycling.

6. **Preview rebuild / display**
   - gamemd: after a successful chooser return, the handler rebuilds map/preview state, may recreate `DAT_00AC1154`, and invalidates the skirmish dialog so `WM_PAINT` can call `DrawStartPositions`.
   - Rust: `selected_map_idx` has no consumer in `app_skirmish_shell_render.rs`; `real_preview_surface_available()` returns `false`, and marker/label projection arrays are empty.
   - Verdict: NOT-IMPLEMENTED. Rust has no real map preview surface update for this shell path.

7. **Timing**
   - gamemd: click release dispatch is synchronous through the Windows message loop; the setup dialog hides before the chooser is shown and is invalidated after chooser completion.
   - Rust: mouse up mutates `selected_map_idx` in the same event handler and keeps drawing the setup shell.
   - Verdict: UNCHECKED. The visible modal transition is absent, but exact frame/message timing was not measured.

8. **Audio**
   - gamemd: owner-draw button `WM_LBUTTONDOWN` plays the shell click sound unless its suppressing state byte is set.
   - Rust: no skirmish-shell-specific button sound call was found on the `dev_skirmish_shell_enabled` mouse path; main-menu shell has a separate sound helper, but this path does not call it.
   - Verdict: UNCHECKED. This is adjacent to the Choose Map action but was not numerically traced in this run.

## Failures

### FAIL 1 - Stage 3: Choose Map opens no chooser

Rust turns `0x5AA` into map cycling:

- `src/ui/skirmish_shell/state.rs:100` maps `OwnerDrawButton::ChooseMap0x5aa` to `SkirmishShellAction::ChooseMap`.
- `src/ui/skirmish_shell/state.rs:165` handles `ChooseMap`.
- `src/ui/skirmish_shell/state.rs:167` sets `selected_map_idx = (selected_map_idx + 1) % maps.len()`.
- `src/app.rs:553` treats `ChooseMap` as a swallowed/no-op after `apply_action` returns `None`.

gamemd's live `FUN_006ACEE0` `0x5AA` branch hides the Skirmish setup window and calls the map chooser routine. This is player-visible every time `Choose Map` is clicked in normal Skirmish setup.

### FAIL 2 - Stage 5: One click changes Rust's selected map index

Rust changes `selected_map_idx` immediately. In this workspace, the configured retail directory has 41 directly visible map files matching the app's map extensions, so default index `0` becomes index `1` after one click.

gamemd does not define `Choose Map` as "next map." It opens a map-selection flow and only updates/rebuilds selection state based on the chooser's result.

## Not Implemented

### NOT-IMPLEMENTED 1 - Stage 6: Real preview rebuild/display after map choice

`app_skirmish_shell_render.rs` does not consume `SkirmishShellState.selected_map_idx` for the preview or scenario text. `real_preview_surface_available()` returns `false`, and start marker sprites/labels are skipped.

gamemd has an active preview object path for offline Skirmish `0x102`: after chooser-related state changes, invalidation leads to `WM_PAINT`, preview blit, `STARTBUT.SHP` markers, and numeric labels through `DrawStartPositions`.

## Adjacent Findings

- Skirmish button click sound parity should be traced separately. The Rust main-menu shell path calls `play_main_menu_button_sound`, but the development Skirmish shell mouse path does not visibly call an equivalent helper.
- Choose-map dialog internals (`FUN_005E68A0`) were not traced beyond proving that `0x5AA` invokes it. A separate trace should cover map list contents, selected-map display, cancel/accept return codes, and preview rebuild values.
- The shell preview trace should verify exact `DAT_00AC1154` source bounds versus Rust `PreviewSection`/`preview_source_bounds` before implementing preview parity.

## Source Pointers

- Rust action mapping: `C:/Users/enok/Documents/ra2-rust-game/src/ui/skirmish_shell/state.rs:97`
- Rust map cycling: `C:/Users/enok/Documents/ra2-rust-game/src/ui/skirmish_shell/state.rs:165`
- Rust action swallowing: `C:/Users/enok/Documents/ra2-rust-game/src/app.rs:531`
- Rust mouse release path: `C:/Users/enok/Documents/ra2-rust-game/src/app.rs:570`
- Rust preview disabled: `C:/Users/enok/Documents/ra2-rust-game/src/app_skirmish_shell_render.rs:458`
- gamemd docs: `C:/Users/enok/Documents/ra2-rust-game-docs/SKIRMISH_SHELL_RETAIL_ASSETS_GHIDRA_REPORT.md:80`, `:122`
- gamemd active path docs: `C:/Users/enok/Documents/ra2-rust-game-docs/MAIN_GAME_STATE_MACHINE_CASES_GHIDRA_REPORT.md:366`
- Fresh read-only Ghidra functions: `FUN_006AE2C0`, `FUN_006AE3F0`, `FUN_006ACEE0`
