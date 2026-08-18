# Skirmish Checkbox Icon vs Label Hit Trace

**Scenario:** Native/dev Skirmish shell dialog `0x102` at 800x600. Click directly on the Short Game checkbox icon, then click the adjacent Short Game label text.

**Scope:** One control, `0x54E` / `GUI:ShortGame`, and only the icon-vs-label hit behavior, checked visual state, and click sound/action ordering.

**Status:** COMPLETE

## Evidence Sources

- Rust input/layout/render: `src/ui/skirmish_shell/layout.rs`, `src/ui/skirmish_shell/state.rs`, `src/app_skirmish_shell_render.rs`, `src/app.rs`.
- Existing verified docs:
  - `SKIRMISH_0X102_COMMON_PARENT_PAINT_GHIDRA_REPORT.md`: standard offline Skirmish creates dialog `0x102` with proc `0x006AE3F0`; active in YR.
  - `SKIRMISH_RESIZE_SHELL_CHILD_CONTROL_0060C0C0_COMPLETE_0X102_POLICY_GHIDRA_REPORT.md`: late `0x102` fixup moves `0x54E/0x693/0x696/0x69A` left by 1 px.
  - `SKIRMISH_SHELL_640X480_FINAL_VISIBLE_LAYOUT_TRACE.md`: final visible `0x54E` rect is `(71,286,150,16)` and icon blit is `(71,286,18,18)`.
  - `SKIRMISH_TEXT_RENDERER_CALLER_PIXEL_CONTRACT_GHIDRA_REPORT.md`: checkbox label rect is control-left + 26; only local `x < 0x12 && y < 0x12` toggles.
  - `SKIRMISH_CHROME_CONTROL_ART_SUBSTITUTIONS_GHIDRA_REPORT.md`: standard checkbox assets are `cue_i.pcx` unchecked and `cce_i.pcx` checked; label clicks do not toggle.
- Fresh read-only Ghidra spot check:
  - `OwnerDraw_Checkbox_006163A0 @ 0x006163A0` decompiled read-only. The live `WM_LBUTTONDOWN` / `WM_LBUTTONDBLCLK` branch gates on local `x < 18 && y < 18`, flips owner state, invalidates, calls `VocClass__PlayAtPos`, then sends parent `WM_COMMAND`.

## Pipeline

`0x102` Skirmish dialog active in standard YR -> `0x54E` owner-draw checkbox -> icon/label geometry -> mouse-down hit gate -> checked state flip or no-op -> paint chooses checked/unchecked PCX -> optional sound and parent command -> later Start/Back packing reads the checkbox state.

## Stage Results

| Stage | gamemd.exe output | Rust output | Verdict |
|---|---|---|---|
| Active path | Offline Skirmish reaches dialog `0x102` and active owner-draw checkbox callback `0x006163A0`. | Scenario is the native/dev Skirmish shell, behind the dev shell path. | PASS for scoped native/dev path; default app path remains outside this trace. |
| Final 800x600 geometry | Short Game `0x54E` final rect `(71,286,150,16)` after `x-1`; icon `(71,286,18,18)`; label starts at `x=97` from `left+26`. | `compute_layout(800,600)` returns rect `(71,286,150,16)`; `checkbox_icon_rect` returns `(71,286,18,18)`; `checkbox_text_rect` starts at `x=97`. | PASS |
| Icon hit gate | Local icon gate is `x < 18 && y < 18`; click at `(71,286)` maps to local `(0,0)` and toggles. | `RectPx::contains` is half-open; icon rect contains `(71,286)` through `(88,303)` and toggles. | PASS |
| Label hit gate | Click at label start `(97,287)` maps to local `x=26`, outside `x < 18`; no toggle, no sound, no parent command. | `handle_option_mouse_down` checks only `checkbox_icon_rect`; `checkbox_text_rect` click does not toggle and returns no action. | PASS |
| Checked visual state selection | Standard variant uses `cue_i.pcx` unchecked and `cce_i.pcx` checked. Icon click flips state before invalidation/repaint. | `checkbox_entry` selects `cue_i`/`cce_i` from `shell.short_game`, and icon click flips `shell.short_game` immediately. | PASS for asset/state selection; final pixel equality UNCHECKED. |
| Icon click sound/action ordering | On icon click, gamemd writes new state, invalidates, calls `VocClass__PlayAtPos`, then sends parent `WM_COMMAND` with the new state in the high word. | Rust writes new state on mouse-down and returns `SkirmishShellAction::None`; no Skirmish checkbox sound call was found in `handle_skirmish_shell_mouse_down` or the option state path. | FAIL |
| Start/Back propagation | Checkbox globals are not rewritten by the click command handler; they are packed later when Start/Back apply reads current checkbox controls. | Rust keeps the option directly in `SkirmishShellState` and packs it into launch options later. No player-visible mismatch was found for this scenario. | PASS |

## Failures

1. **Icon click sound is missing in Rust.**
   - **gamemd:** `OwnerDraw_Checkbox_006163A0` handles `WM_LBUTTONDOWN` / `WM_LBUTTONDBLCLK`; after an in-icon click it toggles state, invalidates, calls `VocClass__PlayAtPos`, then sends parent `WM_COMMAND`.
   - **Rust:** `handle_option_mouse_down` toggles `shell.short_game` and returns `None`; `App::handle_skirmish_shell_mouse_down` only has button press handling and does not call a Skirmish checkbox sound helper.
   - **Player-visible difference:** Clicking the Short Game icon changes the checkmark but is silent in Rust where retail plays a click sound.

## Unchecked

- Final rendered checkbox pixel equality is unchecked. Rust selects the verified `cue_i/cce_i` assets, but this trace did not capture retail and Rust surface pixels; non-flag PCX alpha behavior remains an adjacent audit risk from prior chrome reports.
- Exact sound identity/volume for the checkbox `VocClass__PlayAtPos` call is unchecked. The call site and ordering are verified; this trace did not resolve the final audible sample name from runtime sound tables.

## Adjacent Findings

- Button click sound parity is separate and already covered by the Skirmish button sound report; not traced here.
- Disabled checkbox paint and variant messages `0x4E5/0x4E6` are not part of this standard `0x102` Short Game click scenario.

## Verdict Tally

PASS: 6 | FAIL: 1 | UNCHECKED: 2 | NOT-IMPLEMENTED: 0
