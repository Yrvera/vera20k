# Skirmish Checkbox Click Sound - Ghidra Research Report

**Address(es):** `0x006163A0` (`OwnerDraw_Checkbox_006163A0`), `0x006691E0` / `0x006695C2..0x006695EF` (`RulesClass__ReadAudioVisual` GUICheckboxSound block), `0x00750920` (`VocClass__PlayAtPos`)
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** Sound behavior for Skirmish option checkboxes routed through `OwnerDraw_Checkbox_006163A0`, using Short Game `0x54E` as the concrete sample.
**Non-Scope:** Checkbox PCX visual geometry, label mapping, combo/trackbar/button sounds, and `0x6B` modal control visuals.
**Confidence:** High for callback ordering, sound source field, INI key, and current Rust implementation status; Medium for OS-level disabled-control mouse suppression because the callback itself has no disabled branch but Windows message delivery was not runtime-observed.
**Active in YR:** Yes. Standard Skirmish dialog `0x102` uses owner-draw Button-class checkbox controls; subclass selection routes Button controls with style low bits `(style & 3) == 3` to `OwnerDraw_Checkbox_006163A0` in `FUN_0060f9a0 @ 0x0060FE58..0x0060FE97`.

## 0. Working Notes Required by Swarm Slot

- **Target question:** What exact sound does `OwnerDraw_Checkbox_006163A0` play for a Skirmish option checkbox icon click, and when?
- **Non-goals:** Do not re-audit checkbox PCX geometry, text positioning, or other Skirmish control sound classes.
- **Evidence needed to mark COMPLETE:** Binary callback path, INI reader path, sound field bridge, call arguments, no-sound gates, current Rust mismatch, and implementation handoff.
- **Stop conditions:** Stop after resolving the checkbox click sound slice; list related sounds as open questions instead of investigating them.

## 1. Overview

Skirmish option checkbox clicks use the generic owner-draw checkbox callback. A mouse down or double-click inside the first 18x18 local pixels toggles the checkbox state, invalidates the control, then plays the sound ID stored in the Rules/AudioVisual `GUICheckboxSound` field before sending `WM_COMMAND` to the parent. In stock YR INI, `GUICheckboxSound=MenuClick`, and `[MenuClick]` resolves to `umenucl1`.

Rust now parses the matching `gui_checkbox_sound` rule value, emits a Skirmish shell UI sound event on icon-gated checkbox toggles, drains that event in the app layer, and plays it through the shell UI sound plumbing.

## 2. Class Layout / Key Offsets

| Field / address | Meaning | Evidence | Active in YR |
|---|---|---|---|
| owner-draw control state `+0xE8` from callback local base (`piVar10[0x3a]`) | Checkbox checked state, treated as `1` checked and non-`1` unchecked | `0x0061670E..0x00616722` reads, compares with `1`, writes `SETNZ` result | Yes |
| owner-draw control text ptr `+0x64` (`piVar10[0x19]`) | Label used by paint path only | `OwnerDraw_Checkbox_006163A0` paint branch calls `FUN_00621040` with `piVar10[0x19]` | Yes |
| owner-draw disabled/alternate variant flags `+0xD9`, `+0xDA` | Paint asset variant gates only in this function | message `0x4E5/0x4E6`, paint branch | Conditional; paint variants only, not the click-sound gate |
| Rules/AudioVisual `+0x1AC` (`param_1[0x6b]`) | `GUICheckboxSound` resolved sound index | `RulesClass__ReadAudioVisual @ 0x006691E0`, block `0x006695C2..0x006695EF`, reads `GUICheckboxSound` and writes `[ESI+0x1AC]` | Yes |
| global Rules pointer `DAT_008871e0` plus `+0x1AC` | Runtime source of checkbox sound index for click playback | `OwnerDraw_Checkbox_006163A0 @ 0x00616736..0x0061674E` loads `[DAT_008871e0]+0x1AC` before `VocClass__PlayAtPos` | Yes |

## 3. Core Logic

For `WM_LBUTTONDOWN` (`0x201`) and `WM_LBUTTONDBLCLK` (`0x203`), the callback enters the same local branch:

```text
if local_x < 18 and local_y < 18:
    old_state = checkbox_state
    new_state = (old_state != 1) ? 1 : 0
    checkbox_state = new_state
    InvalidateRect(hwnd, null, false)
    VocClass__PlayAtPos(Rules.AudioVisual.GUICheckboxSound, 1.0, 0x2000, 0)
    parent = GetParent(hwnd)
    id = GetWindowLong(hwnd, GWL_ID)
    SendMessage(parent, WM_COMMAND, low16(id) | (new_state << 16), hwnd)
return 0
```

Verified assembly for the click/sound/send ordering:

- Bounds gate: `0x006166EE..0x00616708` extracts local x/y from `lParam`, compares both against `0x12`, and jumps out when either is `>= 18`.
- Toggle: `0x0061670E..0x00616722` reads old state from `[EBP+0xE8]`, compares to `1`, computes `SETNZ`, and writes the new state before any sound call.
- Invalidation: `0x00616719`, `0x00616720`, `0x0061672F`, `0x00616730` push null rect / erase false / hwnd and call `InvalidateRect`.
- Sound: `0x00616736..0x0061674E` loads global Rules, pushes loop/handle `0`, sets `EDX=0x2000`, pushes float `0x3F800000`, loads `ECX=[Rules+0x1AC]`, and calls `0x00750920`.
- Parent notification: `0x00616753..` begins parent lookup and `SendMessageA`; decompile confirms `WM_COMMAND` `0x111` after sound.

`WM_USER`-style setter `0xF1` sets state and invalidates, but does not play sound. Getter `0xF0` returns state and does not play sound. Variant messages `0x4E5/0x4E6/0x4E7` update/query paint variant flags and do not play sound.

## 4. INI Keys

| Key | Section | Stock value | Binary read | Effect | Active in YR |
|---|---|---|---|---|---|
| `GUICheckboxSound` | `[AudioVisual]` | `MenuClick` in `rules.ini` and `rulesmd.ini` | `RulesClass__ReadAudioVisual @ 0x006691E0`, block `0x006695C2..0x006695EF`, via string `0x0083AAC4` | Sound index stored in Rules `+0x1AC`; used by owner-draw checkbox click playback | Yes |
| `MenuClick` | `[SoundList]`, `[MenuClick]` | ID `299`; `Sounds=umenucl1` in `sound.ini` and `soundmd.ini` | `VocClass__FindByName` resolves the string read from `GUICheckboxSound` | Final audible sample pool for stock YR checkbox click | Yes |

Fallback behavior: the reader preserves the previous field value if `CCINIClass__ReadString` returns no value or `VocClass__FindByName` returns `-1`. Evidence: `0x006695C2` saves old `[ESI+0x1AC]` in `EBX`; `0x006695DB..0x006695ED` falls back to `EBX`; `0x006695EF` writes the chosen value.

## 5. Integration Points

| Integration | Behavior | Evidence | Active in YR |
|---|---|---|---|
| Subclass binding | `FUN_0060f9a0` selects `OwnerDraw_Checkbox_006163A0` for Button class controls where `(style & 3) == 3` | `0x0060FE58..0x0060FE97` decompile | Yes |
| Live click entry | `WM_LBUTTONDOWN` and `WM_LBUTTONDBLCLK` share the same icon gate and sound path | `OwnerDraw_Checkbox_006163A0`, branches `param_2 == 0x201` and `param_2 == 0x203` to `LAB_006166EE` | Yes |
| Sound playback | Click path calls `VocClass__PlayAtPos` with Rules `GUICheckboxSound`, volume `1.0`, `EDX=0x2000`, final stack arg `0` | `0x00616736..0x0061674E` | Yes |
| Parent notification | `WM_COMMAND` is sent after sound with low word control ID and high word new state | decompile after `VocClass__PlayAtPos`; `SendMessageA(hWnd,0x111,uVar2 & 0xffff | new_state << 16, hwnd)` | Yes |

## 6. Current Rust Implementation Status

Rust now matches the researched checkbox sound slice for the native/dev Skirmish shell:

- `src/ui/skirmish_shell/state.rs::handle_option_mouse_down` keeps the icon-only `checkbox_icon_rect(...).contains(x, y)` toggle gate and exposes a checkbox-toggle action rather than treating it as a silent no-op.
- Rules parsing now carries `[AudioVisual] GUICheckboxSound` through `gui_checkbox_sound`, preserving INI override behavior instead of hardcoding stock `MenuClick`.
- The app-level Skirmish shell mouse-down path queues `SkirmishShellUiSound::GuiCheckboxSound` only after a successful icon-gated checkbox toggle.
- The app drains the queued shell UI sound and plays it through the existing shell UI sound plumbing. Setter/getter/variant paths remain silent.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `OwnerDraw_Checkbox_006163A0` click path | verified | decompile `0x006163A0`; assembly `0x006166EE..0x0061674E` | none |
| `WM_LBUTTONDOWN` vs `WM_LBUTTONDBLCLK` | verified | both branch to `LAB_006166EE` in decompile | OS-generated click sequence around double-click not runtime-counted |
| Icon-only sound gate | verified | `x < 0x12 && y < 0x12` before toggle/sound | none |
| Label/outside click no-sound | verified | branch returns `0` when either coordinate is `>= 0x12` | none |
| Setter/getter/variant messages no-sound | verified | `0xF0`, `0xF1`, `0x4E5`, `0x4E6`, `0x4E7` branches | none |
| `RulesClass__ReadAudioVisual` `GUICheckboxSound` reader | verified | decompile `0x006691E0`; assembly `0x006695C2..0x006695EF`; string `0x0083AAC4` | none |
| `VocClass__PlayAtPos` call arguments | verified | assembly `0x00616736..0x0061674E`; decompile `0x00750920` | exact semantic name of `EDX=0x2000` remains treated as playback-position/pan argument, not needed for Rust string-level hook |
| Disabled control OS message suppression | touched-not-exhausted | callback click branch has no disabled-style gate; paint branch checks disabled style | runtime/Win32 delivery for disabled owner-draw checkbox not observed |
| Rust input path | implemented/current | `src/ui/skirmish_shell/state.rs`, `src/app.rs` scan; `gui_checkbox_sound`; `SkirmishShellUiSound::GuiCheckboxSound` | keep regression coverage focused on icon-gated emission and silent non-click paths |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-1 - Is `OwnerDraw_Checkbox_006163A0` live for standard YR Skirmish checkboxes? -> Yes; Button controls with `(style & 3) == 3` are routed to this callback by `FUN_0060f9a0`.` (evidence: `0x0060FE58..0x0060FE97`)
- `[RESOLVED] OQ-2 - Which messages trigger checkbox click sound? -> `WM_LBUTTONDOWN` `0x201` and `WM_LBUTTONDBLCLK` `0x203` share the click branch.` (evidence: `OwnerDraw_Checkbox_006163A0` decompile)
- `[RESOLVED] OQ-3 - What is the hit gate? -> Both local x and local y must be `< 0x12`; equality at 18 is outside.` (evidence: `0x006166EE..0x00616708`)
- `[RESOLVED] OQ-4 - Does label click play sound? -> No; label x starts outside the 18px icon gate and the branch returns before toggle/sound.` (evidence: `0x00616700..0x00616708`)
- `[RESOLVED] OQ-5 - What state is sent to the parent? -> New state after toggle is placed in the `WM_COMMAND` high word.` (evidence: decompile `SendMessageA(..., id | new_state << 16, hwnd)`)
- `[RESOLVED] OQ-6 - Is sound before or after parent notification? -> Before parent notification.` (evidence: `0x0061674E` `VocClass__PlayAtPos` before later `SendMessageA`)
- `[RESOLVED] OQ-7 - Is sound before or after invalidation? -> After `InvalidateRect`.` (evidence: `0x00616730` before `0x0061674E`)
- `[RESOLVED] OQ-8 - Which Rules field supplies the sound? -> Rules/AudioVisual `+0x1AC`, read from `GUICheckboxSound`.` (evidence: `0x006695C2..0x006695EF`, `0x00616736..0x0061674E`)
- `[RESOLVED] OQ-9 - What stock YR sound does that key name? -> `MenuClick`, with `Sounds=umenucl1`.` (evidence: `ini/rulesmd.ini:652`, `ini/soundmd.ini:2926..2927`)
- `[RESOLVED] OQ-10 - What if the INI key is absent or invalid? -> The reader preserves the previous field value.` (evidence: `0x006695C2`, `0x006695DB..0x006695EF`)
- `[RESOLVED] OQ-11 - Do checkbox setter/variant messages play sound? -> No, only invalidation/state/flag updates.` (evidence: `OwnerDraw_Checkbox_006163A0` branches `0xF1`, `0x4E5`, `0x4E6`, `0x4E7`)
- `[RESOLVED] OQ-12 - Does the callback itself check disabled style before playing? -> No disabled-style check is present on the click branch; disabled style is used in paint only.` (evidence: click branch `0x006166EE..0x0061674E`; paint branch reads style and alpha path)
- `[RESOLVED] OQ-13 - Current Rust delta? -> Historical delta is resolved; Rust now parses `gui_checkbox_sound`, queues `SkirmishShellUiSound::GuiCheckboxSound` on icon-gated checkbox toggles, drains it in the app layer, and plays it through shell UI sound plumbing.` (evidence: `src/ui/skirmish_shell/state.rs::handle_option_mouse_down`, `src/app.rs::handle_skirmish_shell_mouse_down`)
- `[DEFERRED] OQ-14 - Does a disabled Win32 checkbox receive click messages in the retail runtime?` (category: `needs-runtime-debugger`; reason: callback has no internal disabled guard, but OS message suppression was not runtime-observed; next-step-if-pursued: set breakpoint on `0x006166EE` while clicking a disabled checkbox variant)
- `[DEFERRED] OQ-15 - Does a physical double-click produce two audible clicks or one in the full Windows message sequence?` (category: `needs-runtime-debugger`; reason: binary proves `WM_LBUTTONDBLCLK` itself plays once when delivered, but full OS down/up/double-click sequence was not counted here; next-step-if-pursued: runtime breakpoint/log around `0x0061674E`)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Icon click toggles first, invalidates/repaints, then plays `GUICheckboxSound`, then parent notification occurs | `0x0061670E..0x0061674E`; decompile `SendMessageA` after sound | implemented/current: icon-gated toggles now surface a checkbox UI sound action | `src/ui/skirmish_shell/state.rs::handle_option_mouse_down`; `src/app.rs::handle_skirmish_shell_mouse_down` | Preserve the checkbox-toggled signal/app-layer hook and play `rules.general.gui_checkbox_sound`/AudioVisual-equivalent immediately after successful icon toggle | Click Short Game icon at `(71,286)` in native shell: state flips and exactly one `MenuClick`/`umenucl1` UI sound is emitted before any parent action handling | Do not play sound for all option mouse-downs; only the 18x18 icon-gated toggle branch should emit |
| Stock sound source is `[AudioVisual] GUICheckboxSound=MenuClick`; `[MenuClick] Sounds=umenucl1` | `RulesClass__ReadAudioVisual @ 0x006691E0`, block `0x006695C2..0x006695EF`; `rulesmd.ini:652`; `soundmd.ini:2926..2927` | implemented/current: Rust parses `gui_checkbox_sound` and routes `SkirmishShellUiSound::GuiCheckboxSound` through shell UI sound playback | Rules parsing surface for `AudioVisual`/general sound fields plus app shell SFX helper | Keep resolving checkbox sound from the parsed rules field, not from a hardcoded `MenuClick` string | Override `GUICheckboxSound=GenericBeep` in test INI fixture and verify checkbox emits `GenericBeep`, while stock emits `MenuClick` | Do not hardcode `MenuClick`; the binary preserves override behavior and fallback semantics |
| Outside-icon label click and state setter/variant messages do not play checkbox sound | `0x006166EE..0x00616708`; branches `0xF0`, `0xF1`, `0x4E5..0x4E7` | implemented/current: Rust label click leaves state unchanged and silent; non-click state paths should remain silent | `src/ui/skirmish_shell/state.rs` tests and app sound hook tests | Keep label clicks silent; keep programmatic state updates silent unless they model delivered mouse click | Click `Short Game` label text at x `97` and assert no state flip and no UI sound; set checkbox state programmatically in unit test and assert no UI sound hook | Do not attach sound to render/invalidated state or to every boolean change |

### Acceptance Test Names / Current Regression Coverage Targets

- `skirmish_checkbox_icon_click_emits_gui_checkbox_sound_after_toggle`
- `skirmish_checkbox_label_click_does_not_emit_sound`
- `skirmish_checkbox_sound_uses_audio_visual_guicheckboxsound_override`

### Negative Facts / Do Not Do

- Do not use `GUIMainButtonSound` for checkbox clicks. Evidence: checkbox callback loads Rules `+0x1AC`; `GUIMainButtonSound` reads into `+0x188` (`param_1[0x62]`) in `RulesClass__ReadAudioVisual`.
- Do not play sound on label clicks. Evidence: local coordinate gate requires both x and y `< 0x12` before sound.
- Do not play sound on `0xF1` programmatic state set or `0x4E5/0x4E6` variant changes. Evidence: those branches invalidate/update only and never call `VocClass__PlayAtPos`.
- Do not hardcode stock `MenuClick`; use the parsed `GUICheckboxSound` value and let stock INI supply `MenuClick`. Evidence: reader calls `CCINIClass__ReadString` then `VocClass__FindByName`, preserving prior value on missing/invalid key.
- Do not route this through `sim/`; this is shell UI behavior and belongs in app/UI/audio plumbing above simulation. Evidence: live path is Win32 owner-draw UI callback, not gameplay tick logic.

### Stale Docs / Follow-up Docs

- No stale existing document claim was found. The prior trace correctly marked the sound identity as unchecked; replacement wording is now: "Checkbox icon clicks play `[AudioVisual] GUICheckboxSound`, stock `MenuClick`/`umenucl1`, after the state write and invalidation and before parent `WM_COMMAND`."

## Sources

- Ghidra: `OwnerDraw_Checkbox_006163A0 @ 0x006163A0`
- Ghidra: `FUN_0060f9a0 @ 0x0060F9A0`, subclass selection `0x0060FE58..0x0060FE97`
- Ghidra: `RulesClass__ReadAudioVisual @ 0x006691E0`, `GUICheckboxSound` block `0x006695C2..0x006695EF`
- Ghidra: `VocClass__PlayAtPos @ 0x00750920`
- INI: `C:/Users/enok/Documents/ra2-rust-game/ini/rulesmd.ini:652`
- INI: `C:/Users/enok/Documents/ra2-rust-game/ini/soundmd.ini:2926..2927`
- Prior trace: `C:/Users/enok/Documents/ra2-rust-game-docs/traces/SKIRMISH_CHECKBOX_ICON_VS_LABEL_HIT_TRACE.md`
- Rust: `C:/Users/enok/Documents/ra2-rust-game/src/ui/skirmish_shell/state.rs`
- Rust: `C:/Users/enok/Documents/ra2-rust-game/src/app.rs`
