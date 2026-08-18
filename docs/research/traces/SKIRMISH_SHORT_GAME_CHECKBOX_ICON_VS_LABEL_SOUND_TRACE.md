# Skirmish Short Game Checkbox Icon vs Label Sound Trace

**Scenario:** Standard Yuri's Revenge Skirmish dialog `0x102` at `800x600`. Click the Short Game checkbox `0x54E` icon area, then click the adjacent label/text area.

**Scope:** One control only: Short Game `0x54E` / `GUI:ShortGame`. This trace compares icon-vs-label hit behavior, state change, visible repaint/invalidation ordering where provable, and `GUICheckboxSound` source/timing.

**Status:** COMPLETE

## Evidence Sources

- Fresh read-only Ghidra checks in this run:
  - `FUN_006AE2C0 @ 0x006AE2C0`: standard offline Skirmish creates/pumps the shell dialog.
  - `FUN_006AE3F0 @ 0x006AE3F0`: Skirmish dialog proc delegates common shell handling, handles `WM_COMMAND`, and is the active dialog proc for this shell path.
  - `OwnerDraw_Checkbox_006163A0 @ 0x006163A0`: owner-draw checkbox click/paint callback; no mutating Ghidra tools were used.
- Verified research:
  - `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_CHECKBOX_CLICK_SOUND_GHIDRA_REPORT.md`
  - `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_0X102_COMPLETE_CHILD_RECT_MATRIX_GHIDRA_REPORT.md`
  - `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_CHECKBOX_CONTROL_LABEL_MAPPING_GHIDRA_REPORT.md`
  - `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_CHECKBOX_TRACKBAR_OWNERDRAW_PAINT_GEOMETRY_GHIDRA_REPORT.md`
- Current Rust source:
  - `C:/Users/enok/Documents/ra2-rust-game/src/ui/skirmish_shell/layout.rs`
  - `C:/Users/enok/Documents/ra2-rust-game/src/ui/skirmish_shell/state.rs`
  - `C:/Users/enok/Documents/ra2-rust-game/src/app.rs`
  - `C:/Users/enok/Documents/ra2-rust-game/src/app_skirmish_shell_render.rs`
  - `C:/Users/enok/Documents/ra2-rust-game/src/rules/ruleset.rs`
  - `C:/Users/enok/Documents/ra2-rust-game/src/sim/game_options.rs`
- INI evidence:
  - `ini/rulesmd.ini:652` / `ini/rules.ini:498`: `GUICheckboxSound=MenuClick`
  - `ini/soundmd.ini:2926..2927` / `ini/sound.ini:3166..3167`: `[MenuClick] Sounds=umenucl1`

## Active YR Confirmation

The scoped gamemd references are active in standard YR, not dormant TS legacy. `FUN_006AE2C0` reaches the offline Skirmish shell, prior reports bind it to dialog resource `0x102` and proc `0x006AE3F0`, and the verified subclass path routes Button controls with style low bits `(style & 3) == 3` to `OwnerDraw_Checkbox_006163A0`. Short Game `0x54E` is one of those active controls.

## Pipeline

`0x102` Skirmish shell -> Short Game child `0x54E` -> owner-draw checkbox rect and label rect -> mouse-down local hit gate -> checkbox state toggle or no-op -> repaint source chooses `cue_i.pcx`/`cce_i.pcx` -> optional `GUICheckboxSound` playback -> parent/action handling -> later Start/Back packs live checkbox state.

## Stage Results

| Stage | gamemd.exe output | Current Rust output | Verdict |
|---|---|---|---|
| Active shell/control path | Standard offline Skirmish uses dialog `0x102`; `0x54E` is an owner-draw Button checkbox routed to `OwnerDraw_Checkbox_006163A0`. | The native/dev Skirmish shell models `SkirmishCheckboxId::ShortGame0x54e` in the dialog `0x102` layout/state/render path. | PASS for this scoped shell path. |
| Initial Short Game state | `[MultiplayerDialogSettings] ShortGame=yes` seeds the control checked before this click, unless user INI overrides it. | `GameOptions::default().short_game == true`; `SkirmishShellState::default()` copies it into `short_game`. | PASS |
| Final 800x600 geometry | `0x54E` raw `(72,286,150,16)` receives the `0x102` `x-1` fixup, final rect `(71,286,150,16)`; icon is local `18x18`; label starts at `x+26 = 97`. | `compute_layout(800,600)` returns Short Game rect `(71,286,150,16)`; `checkbox_icon_rect` is `(71,286,18,18)`; `checkbox_text_rect.x == 97`. | PASS |
| Icon click gate | Click at `(71,286)` maps to local `(0,0)` and satisfies unsigned/local `x < 18 && y < 18`; gate is half-open and excludes equality at `18`. | `RectPx::contains` is half-open; `checkbox_icon_rect(...).contains(71,286)` is true and covers local `0..17` in both axes. | PASS |
| Icon state change | With initial checked state `1`, native computes `new_state = (old_state != 1)`, writes `0`, invalidates, plays sound, then sends parent `WM_COMMAND` with the new state in the high word. | `handle_option_mouse_down` flips `shell.short_game` from `true` to `false`, queues `SkirmishShellUiSound::GuiCheckboxSound`, returns `None`, and app drains the sound immediately in the same mouse-down path. | PASS for state and sound ordering; exact native invalidation-to-frame timing remains UNCHECKED. |
| Label/text click gate | Click at label start `(97,287)` maps to local x `26`, outside the `<18` gate; native returns with no toggle, no invalidation, no sound, no parent command. | `handle_option_mouse_down` checks only `checkbox_icon_rect`; `checkbox_text_rect` click does not toggle and queues no sound. | PASS |
| Checkbox visual source | Standard enabled default path uses `cue_i.pcx` unchecked and `cce_i.pcx` checked; icon click changes the state before repaint. | `checkbox_entry` selects `cue_i`/`cce_i` from `shell.short_game`, and rendering places the icon at `checkbox_icon_rect`. | PASS for asset/state selection; final pixel equality UNCHECKED. |
| Checkbox sound source | Native reads Rules/AudioVisual `GUICheckboxSound`; stock YR value is `MenuClick`, resolving to `[MenuClick] Sounds=umenucl1`. | `GeneralRules::from_ini` parses `GUICheckboxSound`; `App::skirmish_shell_ui_sound_id` maps `GuiCheckboxSound` to `rules.general.gui_checkbox_sound`; stock INI is `MenuClick`. | PASS at sound-id/source level. |
| Checkbox sound timing/gate | Sound fires only after successful icon-gated `WM_LBUTTONDOWN`/double-click, after state write/invalidation and before parent `WM_COMMAND`; label/outside clicks are silent. | Sound is queued only after successful icon hit and drained immediately after `handle_option_mouse_down`; label click queues no sound. No parent action fires for this checkbox click. | PASS for user-visible gate/timing; exact audio mixer volume/pan parity UNCHECKED. |

## Failures

None for this scoped scenario in current Rust. The older `SKIRMISH_CHECKBOX_ICON_VS_LABEL_HIT_TRACE.md` failure that reported missing checkbox sound is stale against current source: `state.rs` now queues `GuiCheckboxSound`, and `app.rs` drains it through `rules.general.gui_checkbox_sound`.

## Unchecked

1. Exact repaint/invalidation frame equality is unchecked. Native calls `InvalidateRect(hwnd, NULL, false)` before playing the sound; Rust mutates shell state and relies on the app render loop to display the changed icon. I did not measure both native and Rust frame presentation times numerically.
2. Final pixel equality of the checked/unchecked icon is unchecked. Current Rust selects the verified PCX entries, but this trace did not capture and compare retail/Rust framebuffer pixels.
3. Exact audio mixer volume/pan equality is unchecked. The sound ID source matches `GUICheckboxSound`, but I did not compute native `VocClass__PlayAtPos` mixer output and Rust `sfx.play_sound` mixer output as literal numeric values.

## Adjacent Findings

- The broader default routing of the pixel Skirmish shell remains outside this trace. This report assumes the scoped `0x102` native/dev Skirmish shell path is active.
- Disabled checkbox variants and messages `0x4E5/0x4E6/0x4E7` are outside this standard enabled Short Game click scenario.
- Button, combo, and trackbar sound behavior are separate traces.

## Verdict Tally

PASS: 8 | FAIL: 0 | UNCHECKED: 3 | NOT-IMPLEMENTED: 0
