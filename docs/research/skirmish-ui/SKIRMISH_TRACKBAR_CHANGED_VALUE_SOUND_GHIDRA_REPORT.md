# Skirmish Trackbar Changed-Value Sound - Ghidra Research Report

**Address(es):** `OwnerDraw_Trackbar_0061D950 @ 0x0061D950`, `RulesClass__ReadAudioVisual @ 0x006691E0`, `VocClass__PlayAtPos @ 0x00750920`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** Standard offline Skirmish dialog `0x102` trackbar changed-value notification and sound behavior for Game Speed `0x529`, Credits `0x511`, and Unit Count `0x50C`.
**Non-Scope:** Trackbar art/pixel geometry beyond sound timing dependencies; non-Skirmish sliders; combo/listbox/button sounds except where they identify the sound field contrast.
**Confidence:** High for changed-value gate, notification payload/order, sound source, and Rust-facing gaps. Medium for exact runtime audio audibility when global sound is disabled because the global audio-enable flag was only observed inside `VocClass__PlayAtPos`.
**Active in YR:** Yes. The three controls are initialized by the standard offline Skirmish `0x102` path in `FUN_006AE6E0`; `FUN_0060F9A0` subclasses class `msctls_trackbar32` to `OwnerDraw_Trackbar_0061D950`.

## 0. Working Notes

- Target question: When, why, and with which sound does a standard Skirmish `0x102` trackbar play audio on changed value?
- Non-goals: Do not re-audit trackbar art geometry, value math already proven by prior traces, or unrelated UI control sound systems.
- Evidence needed to mark COMPLETE: binary evidence for trigger/cadence/order, sound Rules field/INI default, all three trackbar reachability/init paths, and current Rust status.
- Stop conditions: Stop after resolving changed-value sound/notification for `0x529`, `0x511`, and `0x50C`; list other UI sound questions as out-of-scope.

## 1. Overview

The standard Skirmish trackbars use a shared owner-draw callback. On any message that changes the stored relative value, minimum, or range span, the callback invalidates the control, sends a parent `WM_HSCROLL`, and then conditionally plays `[AudioVisual] GenericClick`.

Active in YR: Yes. Evidence: `FUN_006AE6E0 @ 0x006AEB6D..0x006AEBFF` initializes `0x529`, `0x511`, and `0x50C`; `FUN_0060F9A0 @ 0x0060FC60..0x0060FCAC` routes `msctls_trackbar32` to `OwnerDraw_Trackbar_0061D950`.

## 2. Class Layout / Key Offsets

These offsets are from the per-control owner-draw state record (`piVar12` in decompile, `EDI` in final-branch assembly), not a public Win32 structure.

| Offset | Decompile slot | Meaning | Active in YR |
|---:|---:|---|---|
| `+0xE8` | `[0x3A]` | mouse/capture active flag | Yes; written final branch `0x0061E632` |
| `+0xEC` | `[0x3B]` | thumb-drag flag | Yes; mouse-down inside thumb sets it at `0x0061E538` |
| `+0xF0` | `[0x3C]` | range span = max - min | Yes; compared for changed-value gate at `0x0061E615` |
| `+0xF4` | `[0x3D]` | relative current value | Yes; compared for changed-value gate at `0x0061E611` |
| `+0xF8` | `[0x3E]` | minimum value | Yes; compared for changed-value gate at `0x0061E61D` |
| `+0xFC` | `[0x3F]` | pixel thumb offset | Yes; recomputed from value/range |
| `+0x100` | `[0x40]` | step/quantization | Yes; defaulted to `1`, Credits set by message `0x4AB` |
| `+0x104` | `[0x41]` | numeric-display flag / plaque reservation | Yes; defaulted to enabled |
| `+0x108` | `[0x42]` | sound suppression flag | Conditional; message `0x4AE` sets it to `(wParam == 0)` |

## 3. Core Logic

### 3.1 Shared change gate

At `LAB_0061E609`, the callback compares the just-computed value/range state against the previous stored state:

```text
changed = (new_relative != old_relative)
       || (new_span     != old_span)
       || (new_min      != old_min)
```

Evidence: assembly `0x0061E609..0x0061E625` compares `EBX` vs `[EDI+0xF4]`, `EBP` vs `[EDI+0xF0]`, and `ESI` vs `[EDI+0xF8]`; decompile sets `bVar1` true only if one differs.

Active in YR: Yes. This final branch is reached from mouse, range, set-pos, paint, and custom-message paths in the live trackbar callback.

### 3.2 Notification and sound order

When `changed` is true, the order is:

1. Store the updated state fields (`+0xE8`, `+0xEC`, `+0xFC`, `+0xF0`, `+0xF4`, `+0xF8`, `+0x100`, `+0x104`).
2. `InvalidateRect(hwnd, NULL, 0)`.
3. `GetParent(hwnd)`.
4. `SendMessageA(parent, WM_HSCROLL 0x114, (((min + relative) & 0xffff) << 16) | 5, hwnd)`.
5. If the message path permits sound and `+0x108 == 0`, play `GenericClick`.

Evidence: decompile final branch in `OwnerDraw_Trackbar_0061D950`; assembly `0x0061E692..0x0061E6AF` builds `wParam` and calls `SendMessageA`, then `0x0061E6B5..0x0061E6DD` gates and calls `VocClass__PlayAtPos`.

Active in YR: Yes. This is the only changed-value final branch for the live owner-draw trackbar callback.

### 3.3 `WM_HSCROLL` payload

The parent notification uses:

```text
message = 0x114          // WM_HSCROLL
wParam  = (((min + relative) & 0xffff) << 16) | 5
lParam  = trackbar HWND
```

The low word `5` is the scroll code; the high word is the low 16 bits of the current absolute value before later parent/game-setting conversion. For Game Speed, this is the visual position `0..6`, not the stored inverted speed.

Evidence: assembly `0x0061E692..0x0061E6AF`: `ADD ESI, EBX` (`min + relative`), `AND ESI,0xffff`, `SHL ESI,0x10`, `OR ESI,0x5`, `PUSH ESI`, `PUSH 0x114`, then `SendMessageA`.

Active in YR: Yes.

### 3.4 Sound source and call shape

The trackbar changed-value sound is `[AudioVisual] GenericClick`, not `GUICheckboxSound` and not `GUIMainButtonSound`.

Evidence:

- At `0x0061E6C6..0x0061E6DD`, the callback loads `EAX = [0x008871E0]`, then `ECX = [EAX + 0x70C]`, pushes handle/source `0`, sets `EDX = 0x2000`, pushes `1.0f`, and calls `VocClass__PlayAtPos @ 0x00750920`.
- `RULESCLASS_FIELDS.csv` maps `RulesClass + 0x70C` to `AudioVisual.GenericClick`.
- `RulesClass__ReadAudioVisual @ 0x006691E0` reads string key `GenericClick` and stores the resolved sound index at `param_1[0x1C3]`, i.e. byte offset `0x70C`.
- `ini/rulesmd.ini:703` and `ini/rules.ini:577` set `GenericClick=MenuClick`.

Active in YR: Yes. The call is in the live callback and the key exists in stock YR/base rules.

### 3.5 Cadence: every changed value, not only release

Every input path that recomputes a different relative value reaches the final branch and can play sound. During thumb drag, `WM_MOUSEMOVE (0x200)` recomputes from cursor position if `+0xEC != 0`; each mouse move that crosses a quantized step and changes `+0xF4` fires the notification/sound sequence. Repeated mouse moves inside the same quantized value do not fire because the stored relative value is unchanged. Mouse release `WM_LBUTTONUP (0x202)` clears capture/drag state but does not itself change the value, so release alone is silent.

Evidence: decompile branch for `WM_MOUSEMOVE` invalidates when thumb-drag flag is set and falls through to final branch; mouse mapping at `0x0061E545..0x0061E594` updates relative value; final changed gate at `0x0061E609..0x0061E625`; `WM_LBUTTONUP` branch clears state/release capture then reaches final branch with the same value.

Active in YR: Yes.

### 3.6 Direct click vs thumb drag

Mouse-down inside the thumb (`x` in `[thumb_x, thumb_x + 12)`) sets the thumb-drag flag without remapping the value; no sound plays unless a later drag move changes the value. Mouse-down in the active vertical band but outside the thumb remaps immediately; if the remapped value differs, it sends `WM_HSCROLL` and plays `GenericClick` on mouse-down.

Evidence: `0x0061E4F5..0x0061E540` y/thumb gate; `0x0061E545..0x0061E594` outside-thumb remap; final branch `0x0061E609..0x0061E6DD`.

Active in YR: Yes.

### 3.7 Sound suppression flag

The callback supports a custom message `0x4AE`: `piVar12[0x42] = (param_3 == 0)`. Sound plays only if this flag is zero. Thus sending `0x4AE` with `wParam == 0` suppresses future changed-value sounds; sending it with non-zero `wParam` allows sounds again.

Evidence: decompile branch `param_2 == 0x4AE` assigns `[0x42]`; final branch checks `(piVar12[0x42] == 0)` before `VocClass__PlayAtPos`. Assembly `0x0061E6BC..0x0061E6C4` tests `[EDI+0x108]` and skips the sound call when non-zero.

Active in YR: Conditional. The mechanism is live in the callback, but `FUN_006AE6E0` standard initialization for `0x529`, `0x511`, and `0x50C` does not send `0x4AE`; default zero-filled state means sounds are enabled.

### 3.8 Programmatic setup does not play trackbar sounds

`TBM_SETRANGE`-like message `0x406` and `TBM_SETPOS`-like message `0x405` can change state, but the callback sets the local sound-permitted flag false before the final branch on these programmatic paths. Standard initialization therefore does not produce trackbar UI sounds while seeding ranges/positions.

Evidence: decompile sets `bVar2 = false` in the `0x406` and `0x405` branches; final sound guard is `if ((bVar2) && (piVar12[0x42] == 0))`. Assembly `0x0061E486..0x0061E4A8` and `0x0061E59A..0x0061E5C9` store recomputed state then jump to final branch with sound-permitted false.

Active in YR: Yes.

## 4. INI Keys

| Key | Stock YR value | Binary field | Effect in this slice | Active in YR |
|---|---:|---:|---|---|
| `[AudioVisual] GenericClick` | `MenuClick` | `RulesClass + 0x70C` | Trackbar changed-value sound | Yes |
| `[AudioVisual] GUICheckboxSound` | `MenuClick` | `RulesClass + 0x1AC` | Checkbox icon click sound, not trackbar | No for this trackbar slice |
| `[AudioVisual] GUIMainButtonSound` | `MenuClick` | `RulesClass + 0x188` | Button mouse-down sound, not trackbar | No for this trackbar slice |
| `[MultiplayerDialogSettings] GameSpeed` | `1` in `rulesmd.ini` | Rules/session fallback | Stored speed; UI visual position is `6 - stored` | Yes |
| `[MultiplayerDialogSettings] MinMoney` | `5000` | `RulesClass + 0x1480` | Credits min | Yes |
| `[MultiplayerDialogSettings] Money` | `10000` | session/default | Initial credits | Yes |
| `[MultiplayerDialogSettings] MaxMoney` | `10000` | `RulesClass + 0x1488` | Credits max | Yes |
| `[MultiplayerDialogSettings] MoneyIncrement` | `100` | `RulesClass + 0x148C` | Credits quantization step | Yes |
| `[MultiplayerDialogSettings] MinUnitCount` | `0` | `RulesClass + 0x1490` | Unit-count min | Yes |
| `[MultiplayerDialogSettings] UnitCount` | `10` | session/default | Initial unit count | Yes |
| `[MultiplayerDialogSettings] MaxUnitCount` | `10` | `RulesClass + 0x1498` | Unit-count max | Yes |

## 5. Integration Points

| Function / point | Behavior | Active in YR |
|---|---|---|
| `FUN_0060F9A0 @ 0x0060FC60..0x0060FCAC` | Class name `msctls_trackbar32` installs `OwnerDraw_Trackbar_0061D950` | Yes |
| `FUN_006AE6E0 @ 0x006AEB6D..0x006AEBFF` | Initializes `0x529`, `0x511`, `0x50C` ranges/positions/credits step | Yes |
| `OwnerDraw_Trackbar_0061D950 @ 0x0061E609..0x0061E6DD` | Change gate, parent notification, optional `GenericClick` sound | Yes |
| `FUN_006ACEE0 @ 0x006AD730+` | On Start/Back apply, rereads `0x400` positions; game speed stored as `6 - visual` | Yes |
| `VocClass__PlayAtPos @ 0x00750920` | Plays resolved sound index if audio is globally enabled and the sound index resolves to an event | Yes / Conditional on audio enabled |

Tick-cycle note: this is modal shell event-loop behavior, not deterministic match simulation. It occurs before game launch.

## 6. Current Rust Implementation Status

Rust has since implemented the user-input changed-value sound path. The table below preserves which parts of this report were original deltas versus current implemented behavior.

| Area | Current Rust status | Evidence |
|---|---|---|
| Three standard trackbars | Present as `SkirmishTrackbarId::{GameSpeed0x529, Credits0x511, UnitCount0x50c}` | `src/ui/skirmish_shell/state.rs` |
| Drag/click value mapping | Present and previously traced as matching for Credits; same helper is shared for all three | `handle_option_mouse_down`, `handle_option_mouse_move`, `trackbar_mouse_value` |
| Changed-value detection | Implemented for user trackbar value changes; unchanged quantized moves remain silent | `src/ui/skirmish_shell/state.rs`, `src/app.rs` |
| Parent `WM_HSCROLL` equivalent | Historical delta from this report; current Rust routes changed user trackbar values through shell UI action/sound handling rather than Win32 messages | `src/ui/skirmish_shell/state.rs`, `src/app.rs` |
| Trackbar sound | Implemented: changed user trackbar values queue/play `SkirmishShellUiSound::GenericClick` | `src/app.rs` Skirmish handlers |
| GenericClick Rules field | Implemented: `generic_click_sound` is parsed for shell UI sound resolution | `src/rules/ruleset.rs` |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `0x102` reachability of `0x529/0x511/0x50C` | verified | `FUN_006AE6E0 @ 0x006AEB6D..0x006AEBFF` | none |
| owner-draw subclass routing | verified | `FUN_0060F9A0 @ 0x0060FC60..0x0060FCAC` | none |
| final changed-value gate | verified | `0x0061E609..0x0061E625` | none |
| notification payload/order | verified | `0x0061E692..0x0061E6AF` | none |
| trackbar sound source | verified | `0x0061E6C6..0x0061E6DD`; `RULESCLASS_FIELDS.csv`; `RulesClass__ReadAudioVisual` | none |
| repeated drag cadence | verified | `0x0061E545..0x0061E594`, `0x0061E609..0x0061E6DD` | runtime audio capture optional |
| sound suppression byte | verified | custom message `0x4AE`, final test `[+0x108]` | no standard `0x102` sender found |
| disabled-window input behavior | resolved for standard offline `0x102` | `SKIRMISH_TRACKBAR_DISABLED_RUNTIME_ENABLE_FLOW_GHIDRA_REPORT.md`; standard flow does not disable `0x529`, `0x511`, or `0x50C` | disabled paint remains conditional only if an external/nonstandard caller sets `WS_DISABLED` |
| current Rust status | verified by source scan / 2026-05-22 verify-doc slot 4 cleanup | `src/ui/skirmish_shell/state.rs`, `src/app.rs`, `src/rules/ruleset.rs` | no remaining trackbar sound implementation work in this slice |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-01 - Which entry point handles standard Skirmish trackbar input? -> OwnerDraw_Trackbar_0061D950 after class routing from FUN_0060F9A0.` (evidence: `0x0060FC60..0x0060FCAC`)
- `[RESOLVED] OQ-02 - Are all three scoped controls on the same callback? -> Yes, 0x529, 0x511, and 0x50C are `msctls_trackbar32` controls initialized by FUN_006AE6E0.` (evidence: `0x006AEB6D..0x006AEBFF`)
- `[RESOLVED] OQ-03 - What exactly counts as changed? -> Relative value, span, or minimum differs from stored state.` (evidence: `0x0061E609..0x0061E625`)
- `[RESOLVED] OQ-04 - Does a drag sound once or every step? -> Every mouse move that changes the quantized relative value can sound; repeated moves with unchanged value do not.` (evidence: `0x0061E545..0x0061E6DD`)
- `[RESOLVED] OQ-05 - Does mouse release itself play the sound? -> No, release only clears capture/drag unless it also causes a state difference, which ordinary release does not.` (evidence: `WM_LBUTTONUP` branch and final gate in `0x0061D950`)
- `[RESOLVED] OQ-06 - What parent notification is sent? -> `WM_HSCROLL 0x114`, low word `5`, high word low 16 bits of current absolute value, lParam child HWND.` (evidence: `0x0061E692..0x0061E6AF`)
- `[RESOLVED] OQ-07 - Is notification before or after sound? -> Before sound.` (evidence: `SendMessageA` precedes sound guard/call at `0x0061E6AF..0x0061E6DD`)
- `[RESOLVED] OQ-08 - Which sound field is used? -> `[AudioVisual] GenericClick`, `RulesClass + 0x70C`.` (evidence: `0x0061E6C6..0x0061E6DD`, `RULESCLASS_FIELDS.csv`)
- `[RESOLVED] OQ-09 - What is the stock sound ID? -> `MenuClick`.` (evidence: `ini/rulesmd.ini:703`, `ini/rules.ini:577`)
- `[RESOLVED] OQ-10 - Is `GUICheckboxSound` used by trackbars? -> No; checkbox callback uses `+0x1AC`, trackbar callback uses `+0x70C`.` (evidence: `0x0061673C..0x0061674E`, `0x0061E6C6..0x0061E6DD`)
- `[RESOLVED] OQ-11 - Does initialization sound? -> No; programmatic range/setpos paths mark sound-permitted false.` (evidence: `0x0061E486..0x0061E4A8`, `0x0061E59A..0x0061E5C9`)
- `[RESOLVED] OQ-12 - What suppresses trackbar sound? -> Custom message `0x4AE` sets `+0x108`; nonzero suppresses final sound.` (evidence: `0x0061E5D9..0x0061E5E9`, `0x0061E6BC..0x0061E6C4`)
- `[RESOLVED] OQ-13 - Are these TS legacy paths? -> No; standard YR Skirmish dialog initializes and uses them.` (evidence: `FUN_006AE6E0`, prior trace reachability)
- `[RESOLVED] OQ-14 - Which Rust functions currently mutate values? -> `handle_option_mouse_down`, `handle_option_mouse_move`, `set_trackbar_visual_value`.` (evidence: source scan `src/ui/skirmish_shell/state.rs`)
- `[RESOLVED] OQ-15 - Does Rust parse `GenericClick` for shell UI? -> Yes; current Rust parses `generic_click_sound` for shell UI sound resolution.` (evidence: source scan `src/rules/ruleset.rs`)
- `[RESOLVED] OQ-16 - What runtime flow disables these three standard trackbars, if any? -> None in standard offline `0x102`; `FUN_006AE6E0` initializes `0x529`, `0x511`, and `0x50C` enabled, `FUN_006ACEE0` reads them on Start/Back, and row/mode/map side-effect functions only disable row sibling controls or Start validation state.` (evidence: `SKIRMISH_TRACKBAR_DISABLED_RUNTIME_ENABLE_FLOW_GHIDRA_REPORT.md`, `FUN_006AE6E0`, `FUN_006ACEE0`, `FUN_006ADC20`, `FUN_006ACD60`)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust status | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Trackbar change emits parent-like value-change boundary before sound: `WM_HSCROLL`, low word `5`, high word low 16 bits of current absolute value | `0x0061E692..0x0061E6AF` | implemented / historical delta | `src/ui/skirmish_shell/state.rs`, `src/app.rs` | Keep emitting a trackbar-changed action/event only when quantized visual value changes; process it before playing sound | Drag Credits from `10000` to `7500`: app observes one changed-value event carrying `0x511` and absolute value `7500` before sound | Do not play sound from raw mouse movement without checking value changed; proposed test: `skirmish_trackbar_drag_emits_changed_event_before_sound` |
| Trackbar changed-value sound uses `[AudioVisual] GenericClick` (`MenuClick` stock) after notification when sound is not suppressed | `0x0061E6C6..0x0061E6DD`; `RULESCLASS_FIELDS.csv`; `rulesmd.ini:703` | implemented | `src/rules/ruleset.rs`, `src/app.rs`, `src/audio/sfx.rs` plumbing reuse | Keep parsing `GenericClick`, then play it for Skirmish trackbar changed-value events after event processing | Drag Unit Count across two integer positions: two `MenuClick` plays, no play on repeated move within same value | Do not use `GUICheckboxSound` or `GUIMainButtonSound` for trackbars; proposed test: `skirmish_trackbar_changed_value_plays_generic_click` |
| Programmatic setup/range/set-position changes are silent; direct outside-thumb click that changes value is audible | `0x0061E486..0x0061E4A8`, `0x0061E59A..0x0061E5C9`, `0x0061E545..0x0061E594` | implemented for user changed-value sound path / historical delta | `src/ui/skirmish_shell/state.rs` initialization vs mouse-event paths | Keep distinguishing init/programmatic setters from user input; direct click outside thumb should change value and play once if value differs | Initialize Game Speed/Credits/Unit Count: no sound; click Credits rail from `10000` to `7500`: exactly one `GenericClick` | Do not centralize all setters to play audio; proposed test: `skirmish_trackbar_programmatic_setpos_is_silent_but_user_click_sounds` |

## 10. Negative Facts / Do Not Do

- Do not wire trackbar changed-value sound to `GUICheckboxSound`; that field is loaded by checkbox callback `0x0061673C..0x0061674E`, not trackbar.
- Do not wire trackbar changed-value sound to `GUIMainButtonSound`; button callback loads `+0x188`, while trackbar loads `+0x70C`.
- Do not play sound on every mouse move; unchanged quantized value fails the final changed gate.
- Do not play sound on initialization or programmatic set/range messages; those paths set the local sound-permitted flag false.
- Do not play release sound for a normal drag release; release clears capture/drag state and is silent unless some state value unexpectedly changed.

## Remaining Uncertainty

- A runtime/debugger audio capture would confirm perceived audibility under user sound settings, but the binary call path and sound index source are verified.
- No standard `0x102` sender of custom suppression message `0x4AE` was found in this slice; suppression behavior is verified in the callback but not observed in normal Skirmish setup.

## Stale Docs / Follow-up Docs

- Prior trace wording in `docs/research/traces/SKIRMISH_CREDITS_TRACKBAR_CLICK_DRAG_TRACE.md` saying `VocClass__PlayAtPos(1.0, 0)` is underspecified. Replacement wording: "When the quantized trackbar value/range/min changes on a user-input path, gamemd sends parent `WM_HSCROLL 0x114` first, then plays `[AudioVisual] GenericClick` from `RulesClass + 0x70C` via `VocClass__PlayAtPos` with volume `1.0`, handle/source `0`, and `EDX=0x2000`, unless the control sound-suppression byte `+0x108` is nonzero."

## Sources

- Ghidra decompile/read-only: `OwnerDraw_Trackbar_0061D950 @ 0x0061D950`
- Ghidra assembly contexts: `0x0061E486`, `0x0061E4F5`, `0x0061E545`, `0x0061E59A`, `0x0061E609`, `0x0061E6C6`, `0x0061E6DD`
- Ghidra decompile/read-only: `FUN_006AE6E0 @ 0x006AE6E0`
- Ghidra decompile/read-only: `FUN_006ACEE0 @ 0x006ACEE0`
- Ghidra decompile/read-only: `FUN_0060F9A0 @ 0x0060F9A0`
- Ghidra decompile/read-only: `RulesClass__ReadAudioVisual @ 0x006691E0`
- Ghidra decompile/read-only: `VocClass__PlayAtPos @ 0x00750920`
- Docs checked: `SKIRMISH_CREDITS_TRACKBAR_CLICK_DRAG_TRACE.md`, `SKIRMISH_CHECKBOXES_AND_TRACKBARS_GHIDRA_REPORT.md`, `SKIRMISH_CHECKBOX_TRACKBAR_PIXEL_GEOMETRY_GHIDRA_REPORT.md`, `SKIRMISH_BUTTON_CLICK_SOUND_PARITY_GHIDRA_REPORT.md`
- INI checked: `ini/rulesmd.ini`, `ini/rules.ini`
- Rust scanned: `src/ui/skirmish_shell/state.rs`, `src/app.rs`, `src/rules/ruleset.rs`
