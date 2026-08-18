# Skirmish Credits Trackbar Click / Drag / Sound Trace

**Scenario:** Standard offline Skirmish dialog `0x102`, 800x600, Credits trackbar `0x511`. Start from stock/default `10000`, click rail away from the thumb at `(443,318)`, release, press the new thumb at `(436,318)`, drag to `(470,318)`, release.

**Verdict:** PARTIAL. The native hit Y gate, local X/value quantization, thumb rectangle, concrete values, display geometry, and `GenericClick` changed-value sound match for the scoped stock-YR sequence. Two implementation-shape differences remain: Rust does not model the native rail-click capture flag and does not expose a parent `WM_HSCROLL` notification boundary. Exact final rail/plaque/thumb pixels remain UNCHECKED without a retail screenshot/pixel comparison.

**Tally:** PASS: 10 | FAIL: 1 | UNCHECKED: 1 | NOT-IMPLEMENTED: 1

## Sources

- Read-only Ghidra spot-check: `OwnerDraw_Trackbar_0061D950 @ 0x0061D950`.
- Verified gamemd reports:
  - `docs/research/skirmish-ui/SKIRMISH_CHECKBOX_TRACKBAR_PIXEL_GEOMETRY_GHIDRA_REPORT.md`
  - `docs/research/skirmish-ui/SKIRMISH_CHECKBOX_TRACKBAR_OWNERDRAW_PAINT_GEOMETRY_GHIDRA_REPORT.md`
  - `docs/research/skirmish-ui/SKIRMISH_TRACKBAR_CHANGED_VALUE_SOUND_GHIDRA_REPORT.md`
  - `docs/research/SHELL_UI_SOUND_PLAYBACK_PLUMBING_GHIDRA_REPORT.md`
- Rust source:
  - `src/ui/skirmish_shell/layout.rs`
  - `src/ui/skirmish_shell/state.rs`
  - `src/app.rs`
  - `src/app_skirmish_shell_render.rs`
  - `src/rules/ruleset.rs`
- INI source:
  - `ini/rulesmd.ini`
  - `ini/soundmd.ini`

## Active YR Confirmation

This is active in standard Yuri's Revenge Skirmish, not dormant TS legacy. Verified reports tie dialog `0x102` creation to the standard offline Skirmish path, `FUN_0060F9A0` routes `msctls_trackbar32` children to `OwnerDraw_Trackbar_0061D950`, and `FUN_006AE6E0` initializes controls `0x529`, `0x511`, and `0x50C`. The read-only Ghidra spot-check of `OwnerDraw_Trackbar_0061D950` confirms the live callback implements the click/drag mapping, parent `WM_HSCROLL`, and `GenericClick` sound branch. The generic `SliderClass` plus/minus SHP path is not used by these controls.

## Pipeline

Mouse down on Credits rail -> native Y gate -> outside-thumb remap -> changed-value gate -> invalidate / parent notification / `GenericClick` -> release -> mouse down on new thumb -> thumb-drag state -> mouse move quantization -> changed-value gate -> invalidate / parent notification / `GenericClick` -> release -> repaint thumb/value text.

## Stage Results

| Stage | gamemd result | Rust result | Verdict |
|---|---|---|---|
| Active control path | Dialog `0x102`, control `0x511`, callback `OwnerDraw_Trackbar_0061D950`; active in standard YR. | `SkirmishTrackbarId::Credits0x511` is handled in `handle_option_mouse_down/move/up`. | PASS |
| Stock data | `MinMoney=5000`, `Money=10000`, `MaxMoney=10000`, `MoneyIncrement=100`; `GenericClick=MenuClick`. | `CREDITS_MIN=5000`, `CREDITS_MAX=10000`, `CREDITS_STEP=100`; default `starting_credits=10000`; `generic_click_sound` parses `GenericClick`. | PASS |
| 800x600 rect | Credits rect `[404,314,128,21]`; active width `128 - 50 - 13 = 65`. | `compute_layout(800,600).trackbars.credits == RectPx::new(404,314,128,21)`; active width helper returns `65`. | PASS |
| Native Y gate | Trackbar input runs only when local `y > 3` and `y < 21`; scenario local y is `318 - 314 = 4`. | `trackbar_mouse_allowed_y` tests `mouse_y > rect.y + rect.h - 18 && mouse_y < rect.y + rect.h`; y `318` passes. | PASS |
| Initial thumb / rail click hit | At `10000`, pixel offset `(10000 - 5000) * 65 / 5000 = 65`; thumb `[470,314,12,21]`; click `(443,318)` is outside the thumb but inside the control. | Same offset and thumb rect via `trackbar_pixel_offset` / `trackbar_thumb_rect`; `rect.contains(443,318)` is true and thumb hit is false. | PASS |
| Rail click value | Native local x after bias: `443 - 404 - 6 = 33`, clamped `[1,66]`; raw `((33 - 1) * 5001) / 65 = 2462`; absolute quantized value `((5000 + 2462) / 100) * 100 = 7400`. | `trackbar_mouse_value` computes `7400`; `set_trackbar_visual_value_if_changed` writes `starting_credits = 7400`. | PASS |
| Rail click sound | Changed value causes one final-branch sound if suppression byte is zero: `[AudioVisual] GenericClick`, stock `MenuClick`, after native notification. | `handle_option_mouse_down` queues one `SkirmishShellUiSound::GenericClick`; `app.rs` drains it and resolves `general.generic_click_sound`. | PASS |
| Rail click capture state | Native `WM_LBUTTONDOWN` sets capture/active state `+0xE8 = 1`; because click is outside the thumb, thumb-drag `+0xEC` remains `0`. | Rust changes the value but leaves `trackbar_drag = None`; no equivalent rail-click capture state is represented. | FAIL |
| Rail release | Native `WM_LBUTTONUP` clears capture/drag; value is unchanged, so release is silent. | `handle_option_mouse_up` clears drag/dropdown state; no sound is queued. | PASS |
| New thumb press | At `7400`, pixel offset `(7400 - 5000) * 65 / 5000 = 31`; thumb `[436,314,12,21]`; press `(436,318)` starts thumb drag without value change or sound. | Same thumb rect; `trackbar_drag = Some(Credits0x511, dragging_thumb=true)` and no pending sound. | PASS |
| Thumb drag value/sound | Move x `470`: local x `60`; raw `((60 - 1) * 5001) / 65 = 4539`; absolute quantized value `9500`; one changed-value `GenericClick`. | `handle_option_mouse_move` computes `9500`, writes `starting_credits = 9500`, queues one `GenericClick`, and app drains/plays it. | PASS |
| Parent notification boundary | Native sends parent `WM_HSCROLL 0x114` with low word `5`, high word current absolute value: `0x1CE80005` for `7400`, `0x251C0005` for `9500`; this precedes sound. | Rust mutates shell state directly and queues sound; there is no explicit parent notification/action carrying `(id=0x511,value)` before sound. | NOT-IMPLEMENTED |
| Release after drag | Native release clears capture/drag and is silent because value did not change. | `handle_option_mouse_up` clears `trackbar_drag` and queues no sound. | PASS |
| Final display geometry and exact pixels | At `9500`, pixel offset `(9500 - 5000) * 65 / 5000 = 58`; thumb x `404 + 1 + 58 = 463`; value text rect `[483,314,49,21]`; exact rail/plaque/thumb raster needs screenshot validation. | Rust computes the same thumb x and value text rect, and displays `"9500"`; no retail screenshot/pixel comparison was run. | UNCHECKED |

## Failures

### 1. Rail-click capture state is not represented

**Stage:** Rail click capture state  
**Player-visible difference:** Usually low. A rail click that changes the value and is then released produces the same value and sound, but gamemd still holds native capture until release while Rust leaves no rail-click drag/capture state.  
**Rust:** `src/ui/skirmish_shell/state.rs:414`  
**gamemd evidence:** `OwnerDraw_Trackbar_0061D950` `WM_LBUTTONDOWN` branch sets capture/active state before outside-thumb remap; verified by read-only Ghidra and owner-draw report.

## Not Implemented

### 1. Parent `WM_HSCROLL` notification boundary is absent

**Stage:** Parent notification boundary  
**Player-visible difference:** Low in this concrete shell because Rust stores the value directly and plays the sound, but the native event ordering boundary is missing. If parent-side Skirmish logic later depends on per-change notifications, Rust will not match that message cadence.  
**Rust:** `src/ui/skirmish_shell/state.rs:416`, `src/app.rs:766`  
**gamemd evidence:** `OwnerDraw_Trackbar_0061D950` final branch sends `WM_HSCROLL 0x114`, low word `5`, high word current absolute value, then calls the `GenericClick` sound path.

## Timing And Ordering

- Rail click changed value: native updates state, invalidates, sends `WM_HSCROLL`, then plays `GenericClick`; Rust updates state, queues `GenericClick`, then app drains it after the mouse-down handler returns.
- Thumb press: no value change and no sound in both.
- Thumb drag across a new quantized value: one changed value and one `GenericClick` in both.
- Release after unchanged value: silent in both.

## Adjacent Findings

- Rust hardcodes stock `5000..10000` and step `100` in the Skirmish shell state/render path. This concrete stock-YR scenario matches, but modded `[MultiplayerDialogSettings]` parity will need data-driven min/max/step.
- Exact final primitive rail/plaque/thumb pixels remain a screenshot-validation task. The native path uses primitive bevel calls plus `trakgrip.pcx` and `trof*.pcx`; this trace only proves the concrete geometry and values.
- Older docs that describe Rust trackbar sound as missing are stale; current source queues and plays `SkirmishShellUiSound::GenericClick` for changed user trackbar values.
