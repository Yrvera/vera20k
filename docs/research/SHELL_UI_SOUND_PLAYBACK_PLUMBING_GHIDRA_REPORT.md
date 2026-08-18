# Shell UI Sound Playback Plumbing - Ghidra Research Report

**Address(es):** `0x00750920`, `0x00669300`, `0x00612B70`, `0x006163A0`, `0x0061D950`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** app-layer playback contract for shell/UI calls that use `VocClass__PlayAtPos`: argument semantics relevant to non-spatial UI playback, missing/empty/unknown INI sound behavior, stock `[AudioVisual]` defaults for `GUIMainButtonSound` and `GenericClick`, and the Rust mapping to `SfxPlayer`/`SoundRegistry`/`audio_indices`.  
**Non-Scope:** individual button/checkbox/trackbar callback parity beyond cited call facts, combo/dropdown callback sound behavior, wider world/spatial sound math, runtime mixer priority/limit eviction, and Rust implementation patches.  
**Confidence:** High for the binary reader fallback, call-site source fields, null-handle behavior, stock INI defaults, and Rust surface scan; Medium for exact pan numeric meaning because decompilation shows the call argument and `SetPan` use but not a named enum.  
**Active in YR:** Yes. The verified call sites are the standard shell owner-draw controls used by YR shell/skirmish dialogs, with no TS-only gate on the inspected paths.

## Working Notes Gate

- **Target question:** What app-layer contract should Rust use for shell UI sounds invoked through `VocClass__PlayAtPos`, especially rule-field lookup, empty/missing behavior, and non-spatial playback mapping?
- **Non-goals:** Do not re-audit each control callback, do not inspect combo sounds beyond open questions, do not modify Rust, INI, or Ghidra state.
- **Evidence needed to mark COMPLETE:** binary evidence for `VocClass__PlayAtPos`, binary evidence for `[AudioVisual]` reader behavior for the relevant keys, INI defaults, Rust audio surface scan, and at least one implementation handoff.
- **Stop conditions:** stop after the app-layer playback contract is resolved; defer exact combo callback use and runtime audible double-click capture to sibling reports/runtime testing.

## 1. Overview

Shell UI sounds are stored in `RulesClass` as resolved `VocClass` indices, not as strings at the point of playback. The owner-draw shell controls pass those indices into `VocClass__PlayAtPos` with volume `1.0f`, pan/source value `0x2000`, and a null handle, so Rust should map these to immediate app-layer SFX playback and keep the simulation layer untouched.

The important fallback rule is in the `[AudioVisual]` reader: if a key is missing, empty, or names an unknown sound, `gamemd.exe` preserves the existing rules field rather than replacing it with "no sound". Stock YR sets both `GUIMainButtonSound=MenuClick` and `GenericClick=MenuClick`; `MenuClick` resolves to `umenucl1` at `Volume=60` in both `soundmd.ini` and base `sound.ini`.

## 2. Class Layout / Key Offsets

| Field / global | Meaning | Evidence | Active in YR |
|---|---|---|---|
| `RulesClass + 0x188` / `param_1[0x62]` | `[AudioVisual] GUIMainButtonSound`, stored as resolved `VocClass` index | `RulesClass__ReadAudioVisual 0x00669300`, string xref `0x0083AB5C`, `RULESCLASS_FIELDS.csv` | Yes: read by the YR `[AudioVisual]` loader and used by shell button mouse-down |
| `RulesClass + 0x1AC` / `param_1[0x6B]` | `[AudioVisual] GUICheckboxSound`, stored as resolved `VocClass` index | `RulesClass__ReadAudioVisual 0x00669300`, string xref `0x0083AAC4`, `RULESCLASS_FIELDS.csv` | Yes: read by the YR loader and used by checkbox callbacks per sibling trace |
| `RulesClass + 0x70C` / `param_1[0x1C3]` | `[AudioVisual] GenericClick`, stored as resolved `VocClass` index | `RulesClass__ReadAudioVisual 0x00669300`, string xref `0x0083A444`, `RULESCLASS_FIELDS.csv` | Yes: read by the YR loader and used by button paint transition and trackbar changed-value paths |
| `DAT_00B1D388` | count of registered `VocClass` entries | `VocClass__PlayAtPos 0x00750920` bounds check | Yes: live playback bounds gate |
| `DAT_00B1D37C` | array/vector backing resolved `VocClass` pointers | `VocClass__PlayAtPos 0x00750920` lookup | Yes: live playback lookup |
| `DAT_008464AC` | audio-enabled global gate | `VocClass__PlayAtPos 0x00750920` first branch | Yes: if zero, playback returns `0` |

## 3. Core Logic

### 3.1 `[AudioVisual]` sound-key reader

`RulesClass__ReadAudioVisual` reads shell/UI sound keys with the same pattern:

1. Save the current field value into a local.
2. Read the INI string from `[AudioVisual]` into a 0x80-byte local buffer using empty-string default `DAT_00889F64`.
3. If `CCINIClass__ReadString` reports no value, keep the old field.
4. Otherwise call `VocClass__FindByName`.
5. If the returned index is `-1`, keep the old field.
6. Otherwise store the returned `VocClass` index back into the field.

This means missing keys, empty values, and unknown sound IDs do **not** actively clear the sound field in `gamemd.exe`; they preserve whatever prior/default value the rules object already held.

**Evidence:** `RulesClass__ReadAudioVisual @ 0x00669300`, with `GUIMainButtonSound` at `0x0066937x..0x0066939x`, `GUIComboOpenSound`/`GUIComboCloseSound`/`GUICheckboxSound` in the same reader block, and `GenericClick` at `0x0066AD31` region. `VocClass__FindByName @ 0x007514D0` returns `-1` when name lookup fails.  
**Active in YR:** Yes. This is the standard YR `[AudioVisual]` rules reader; no TS-only gate appears in this block.

### 3.2 `VocClass__PlayAtPos` non-spatial shell contract

`VocClass__PlayAtPos` is called with the sound index in the implicit `thiscall` register, volume `1.0f` (`0x3F800000`) on the stack, a null handle/source pointer, and the shell call sites set `EDX = 0x2000` per prior button-sound disassembly. The decompiled function:

- returns `0` immediately when the global audio-enabled byte is zero;
- rejects negative or out-of-range sound indices by leaving the resolved event pointer at `0`;
- resolves valid indices through `DAT_00B1D37C[index]`;
- because the shell calls pass handle/source `0`, it skips handle validation/reuse and does not set a loop handle;
- allocates a sound event only when the resolved event pointer is non-null;
- calls `SoundEvent__SetVolume` and `SoundEvent__SetPan`;
- returns the allocated/reused sound event pointer or `0`.

For app-layer Rust parity, the important output-determining result is: a valid configured UI sound starts immediately with normal event volume; invalid/empty/missing effective indices silently do nothing; shell UI calls should not be treated as map-positioned audio.

**Evidence:** `VocClass__PlayAtPos @ 0x00750920`; button call-site register/stack facts in `skirmish-ui/SKIRMISH_BUTTON_CLICK_SOUND_PARITY_GHIDRA_REPORT.md` (`0x00613759..0x00613771`, `0x00613273..0x00613289`).  
**Active in YR:** Yes. Owner-draw shell button, checkbox, and trackbar paths call this live function.

### 3.3 Current Rust mapping

Rust already has the correct broad audio surface for this app-layer contract:

- `AppState` carries `sfx_player`, `sound_registry`, `audio_indices`, `asset_manager`, and loaded `rules`.
- App playback helpers read the parsed shell UI sound fields and call `SfxPlayer::play_sound(sound_id, registry, assets, audio_indices)` without entering `sim/`.
- `SfxPlayer::play_sound` resolves through `SoundRegistry`, applies per-sound volume and SFX master volume, falls back to direct `audio.bag` lookup, and returns `false` without crashing if resolution fails.
- `load_sound_registry` loads `soundmd.ini` first and merges `sound.ini` fallback, matching the YR-first rule.

As of the 2026-05-22 post-implementation source scan, Rust now models the relevant shell UI rule fields for this report: `generic_click_sound`, `gui_checkbox_sound`, `gui_combo_open_sound`, and `gui_combo_close_sound` are present alongside `gui_main_button_sound`, and Skirmish shell UI sound events drain through app-layer playback helpers. The previously recorded status that only `gui_main_button_sound` existed is historical.

Rust still may not fully model the binary reader's preserve-old-value fallback for missing, empty, or unknown sound keys; keep that caveat separate from the now-implemented field plumbing.

**Evidence:** `src/app.rs`, `src/audio/sfx.rs`, `src/app_transitions.rs`, `src/rules/ruleset.rs`, `src/ui/skirmish_shell/state.rs`; 2026-05-22 verify-doc slot 1 audited 22 claims and found 0 wrong claims, with YELLOW status only for stale Rust implementation wording.  
**Active in YR:** Not a binary fact; current Rust implementation status.

## 4. INI Keys

| INI key | Stock YR value | Stock sound entry | Binary field | Binary missing/empty/unknown behavior | Active in YR |
|---|---|---|---|---|---|
| `[AudioVisual] GUIMainButtonSound` | `MenuClick` (`ini/rulesmd.ini:643`) | `[MenuClick] Sounds=umenucl1, Volume=60` (`ini/soundmd.ini:2926`) | `+0x188` / index `0x62` | preserve previous value | Yes |
| `[AudioVisual] GenericClick` | `MenuClick` (`ini/rulesmd.ini:703`) | `[MenuClick] Sounds=umenucl1, Volume=60` (`ini/soundmd.ini:2926`) | `+0x70C` / index `0x1C3` | preserve previous value | Yes |
| `[AudioVisual] GUICheckboxSound` | `MenuClick` (`ini/rulesmd.ini:652`) | `[MenuClick] Sounds=umenucl1, Volume=60` (`ini/soundmd.ini:2926`) | `+0x1AC` / index `0x6B` | preserve previous value | Yes |
| `[AudioVisual] GUIComboOpenSound` | `MenuACBOpen` (`ini/rulesmd.ini:650`) | sound entry in sound INI registry | `+0x1A4` / index `0x69` | preserve previous value | Yes as a rules field; callback use deferred to slot 4 |
| `[AudioVisual] GUIComboCloseSound` | `MenuACBClose` (`ini/rulesmd.ini:651`) | sound entry in sound INI registry | `+0x1A8` / index `0x6A` | preserve previous value | Yes as a rules field; callback use deferred to slot 4 |

Base `ini/rules.ini` carries the same listed defaults for these UI keys, so no YR override conflict was found.

## 5. Integration Points

| Point | Verified behavior | Evidence | Active in YR |
|---|---|---|---|
| owner-draw button mouse down / double-click | uses `GUIMainButtonSound` via `VocClass__PlayAtPos`, null handle, volume `1.0f` | `OwnerDraw_Button_00612B70`, sibling report `0x00613759..0x00613771` | Yes |
| owner-draw button paint transition | uses `GenericClick` on first enabled `'u' -> 'd'` paint transition | `OwnerDraw_Button_00612B70`, sibling report `0x00613273..0x00613289` | Yes, conditional on state transition |
| checkbox icon click | toggles, invalidates, plays a UI sound, sends parent `WM_COMMAND` | `OwnerDraw_Checkbox_006163A0 @ 0x006166EE..0x00616708`, sibling trace | Yes |
| trackbar changed value | invalidates, sends `WM_HSCROLL`, then plays `GenericClick` only if sound gate allows | `OwnerDraw_Trackbar_0061D950`, `LAB_0061E609`, sibling trace | Yes, conditional on value change and sound gate |
| Rust Skirmish shell | historical note: mouse handlers previously mutated state without a shell UI sound helper; current Rust now drains Skirmish shell UI sound requests through app-layer playback helpers | `src/app.rs`, `src/ui/skirmish_shell/state.rs` | Rust status only |

No evidence was found that these UI calls need `sim/` integration. They are shell/app input and paint side effects.

## 6. Current Rust Implementation Status

| Surface | Status | Evidence |
|---|---|---|
| app-layer SFX player | present | `src/audio/sfx.rs:156` |
| YR-first `soundmd.ini`/`sound.ini` registry | present | `src/app_transitions.rs:302` |
| YR-first audio bag indices | present | `src/app_transitions.rs:334` |
| main-menu `GUIMainButtonSound` helper | present | `src/app.rs` |
| Skirmish owner-draw button sound plumbing | present | `src/app.rs`, `src/ui/skirmish_shell/state.rs` |
| parsed `GenericClick` rules field | present as `generic_click_sound` | `rg GenericClick src/rules src`; current Rust source scan |
| parsed checkbox/combo UI sound rules fields | present as `gui_checkbox_sound`, `gui_combo_open_sound`, and `gui_combo_close_sound` | `rg GUICheckboxSound|GUICombo src/rules src`; current Rust source scan |
| binary-style preserve-old-value on missing/unknown keys | still open unless separately verified; do not assume full preserve-old semantics from field presence alone | `RulesClass__ReadAudioVisual 0x00669300`; current Rust source scan |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `VocClass__PlayAtPos` valid index path | verified | `0x00750920` | exact mixer priority/limit internals out of scope |
| `VocClass__PlayAtPos` audio-disabled path | verified | `0x00750920` first branch on `DAT_008464AC` | none for app helper |
| `VocClass__PlayAtPos` invalid index/null event path | verified | `0x00750920` bounds and null checks | none for app helper |
| `VocClass__PlayAtPos` null handle behavior | verified | `0x00750920`, shell call sites pass `0` | exact non-null handle users out of scope |
| `RulesClass__ReadAudioVisual` `GUIMainButtonSound` | verified | `0x00669300`, string `0x0083AB5C`, `RULESCLASS_FIELDS.csv` | none |
| `RulesClass__ReadAudioVisual` `GenericClick` | verified | `0x00669300`, string `0x0083A444`, `RULESCLASS_FIELDS.csv` | none |
| `RulesClass__ReadAudioVisual` checkbox/combo fields | verified as reader fields | `0x00669300`, `RULESCLASS_FIELDS.csv` | exact combo callback use handled by sibling slot |
| stock YR INI defaults | verified | `ini/rulesmd.ini`, `ini/soundmd.ini` | none |
| Rust app audio plumbing | verified by source scan | `src/app.rs`, `src/audio/sfx.rs`, `src/app_transitions.rs`, `src/ui/skirmish_shell/state.rs` | implemented for shell UI sound field plumbing; preserve-old fallback caveat remains |
| runtime audible double-trigger behavior for button mouse-down + paint transition | deferred | sibling report notes both call sites | runtime capture would settle coalescing/audibility |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-001 - Which mode applies? -> exhaustive-slice for app-layer playback plumbing, not each control callback.` (evidence: `SKILL.md` scope classification plus bounded target)
- `[RESOLVED] OQ-002 - Which binary playback function is in scope? -> VocClass__PlayAtPos.` (evidence: `0x00750920`, sibling reports)
- `[RESOLVED] OQ-003 - Is shell UI playback active in YR? -> Yes, owner-draw shell controls call this function with no TS-only gate found.` (evidence: `OwnerDraw_Button_00612B70`, `OwnerDraw_Checkbox_006163A0`, `OwnerDraw_Trackbar_0061D950`)
- `[RESOLVED] OQ-004 - What happens when global audio is disabled? -> returns 0 before lookup/allocation.` (evidence: `VocClass__PlayAtPos 0x00750920`)
- `[RESOLVED] OQ-005 - What happens for negative/out-of-range indices? -> no event pointer is resolved and no sound is allocated.` (evidence: `VocClass__PlayAtPos 0x00750920`)
- `[RESOLVED] OQ-006 - What does null handle/source do? -> skips validation/reuse and skips loop-handle set; shell calls pass null.` (evidence: `VocClass__PlayAtPos 0x00750920`, sibling button call ranges)
- `[RESOLVED] OQ-007 - Which rule key backs main shell button mouse-down? -> GUIMainButtonSound at `+0x188`, default MenuClick.` (evidence: `RulesClass__ReadAudioVisual 0x00669300`, `ini/rulesmd.ini:643`)
- `[RESOLVED] OQ-008 - Which rule key backs button paint-transition and trackbar changed-value sound? -> GenericClick at `+0x70C`, default MenuClick.` (evidence: `RulesClass__ReadAudioVisual 0x00669300`, `ini/rulesmd.ini:703`, sibling traces)
- `[RESOLVED] OQ-009 - Does missing/empty/unknown UI sound key clear the sound? -> No, the reader preserves the previous field on no read or `FindByName == -1`.` (evidence: `RulesClass__ReadAudioVisual 0x00669300`, `VocClass__FindByName 0x007514D0`)
- `[RESOLVED] OQ-010 - What stock sound asset does MenuClick select? -> `umenucl1` at `Volume=60`.` (evidence: `ini/soundmd.ini:2926`, `ini/sound.ini:3166`)
- `[RESOLVED] OQ-011 - Does Rust already have a non-spatial SFX surface? -> Yes, `SfxPlayer::play_sound`.` (evidence: `src/audio/sfx.rs:156`)
- `[RESOLVED] OQ-012 - Does Rust parse GenericClick? -> Historical scan found no field; current Rust now has `generic_click_sound`.` (evidence: `rg GenericClick src/rules src`; current Rust source scan)
- `[RESOLVED] OQ-013 - Does Rust parse GUICheckboxSound / GUIComboOpenSound / GUIComboCloseSound? -> Historical scan found no fields; current Rust now has `gui_checkbox_sound`, `gui_combo_open_sound`, and `gui_combo_close_sound`.` (evidence: `rg GUICheckboxSound|GUICombo src/rules src`; current Rust source scan)
- `[RESOLVED] OQ-014 - Should this enter `sim/`? -> No; all verified call sites are shell/app UI side effects.` (evidence: owner-draw callbacks and current `AppState` audio ownership)
- `[DEFERRED] OQ-015 - Do combo open/close/select callbacks actively play their parsed GUI combo fields?` (category: `out-of-scope`; reason: slot 4 owns combo/dropdown/scrollbar callbacks; next-step-if-pursued: read slot 4 report)
- `[DEFERRED] OQ-016 - Can runtime message coalescing make the button paint-transition `GenericClick` inaudible on some clicks?` (category: `needs-runtime-debugger`; reason: binary proves the call site and gate but not final audible mixing in every OS message order; next-step-if-pursued: capture retail click audio/event trace)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Shell UI `VocClass__PlayAtPos` calls are app-layer, non-spatial, null-handle playback of resolved rule sounds; invalid/missing effective sounds silently do nothing | `VocClass__PlayAtPos 0x00750920`; button call ranges `0x00613759..0x00613771`, `0x00613273..0x00613289` | Historical delta was missing Skirmish playback; current Rust now drains Skirmish shell UI sound requests through app-layer helpers | `src/app.rs`, `src/audio/sfx.rs`, `src/ui/skirmish_shell/state.rs` | Maintain app-layer shell UI sound helper(s) that call `SfxPlayer::play_sound` through existing registry/assets/indices; no `sim/` dependency | Press a Skirmish owner-draw button with stock assets and observe one `MenuClick` SFX attempt without needing map coordinates; proposed regression test: `skirmish_button_mouse_down_plays_gui_main_button_sound` | Do not route through positional/spatial volume APIs or deterministic sim events |
| `GenericClick` is a separate `[AudioVisual]` field at `RulesClass + 0x70C`; stock default is also `MenuClick` but it is not semantically the same field as `GUIMainButtonSound` | `RulesClass__ReadAudioVisual 0x00669300`, `ini/rulesmd.ini:703`, sibling button/trackbar traces | Historical delta was no parsed `generic_click` field; current Rust now has `generic_click_sound` | `src/rules/ruleset.rs`, `src/app.rs`, Skirmish shell render/input state | Keep `GenericClick` stored separately and use it for paint-transition and changed-value paths that verified it | With a test rules snippet setting `GUIMainButtonSound=A` and `GenericClick=B`, button mouse-down uses A while transition/value-change helper selects B; proposed regression test: `generic_click_rule_is_distinct_from_gui_main_button_sound` | Do not alias `GenericClick` to `GUIMainButtonSound` just because stock INI sets both to `MenuClick` |
| The binary rules reader preserves the previous sound index when a UI sound key is missing, empty, or names an unknown sound | `RulesClass__ReadAudioVisual 0x00669300`; `VocClass__FindByName 0x007514D0` | Full Rust preserve-old behavior is still an open caveat unless separately verified | `src/rules/ruleset.rs`, sound-rule parsing/tests | For stock parity, at minimum keep stock defaults loaded; for mod parity, consider rule-level fallback semantics so empty/unknown overrides do not clear an existing/default UI sound | Parse defaults plus an override where `GenericClick=` is empty and verify the effective value remains default when modeling binary fallback; proposed test: `audio_visual_sound_key_empty_preserves_existing_default` | Do not treat empty strings as an intentional retail mute unless a future binary slice proves a different clear mechanism |

### Negative Facts / Do Not Do

- Do not put shell UI sound playback in `sim/`; verified paths are owner-draw Win32 shell callbacks and `AppState` already owns audio playback.
- Do not use `play_sound_with_volume` or map-distance attenuation for shell UI calls; `VocClass__PlayAtPos` call sites pass null handle and no world coordinate, while `VocClass__PlayAt`/`PlayAtCoord` are separate spatial paths.
- Do not collapse `GenericClick` into `GUIMainButtonSound`; they are distinct Rules offsets (`+0x70C` vs `+0x188`) even though stock YR assigns both `MenuClick`.
- Do not make missing, empty, or unknown `[AudioVisual]` sound IDs actively clear a previous/default sound without further evidence; the binary reader preserves the old value.
- Do not use `ShellButtonSlideSound` for Skirmish click/changed-value sounds; prior click report verified the click path uses `GUIMainButtonSound` and `GenericClick`, while `ShellButtonSlideSound` is separate and stock-empty.

### Remaining Uncertainty

- Exact combo/dropdown callback sound use is intentionally deferred to slot 4.
- Exact audible behavior when a button mouse-down `GUIMainButtonSound` and a paint-transition `GenericClick` both target `MenuClick` may require a retail runtime capture to know whether users hear one or two events in a given message order.
- Exact symbolic meaning of the `EDX = 0x2000` pan/source argument remains unnamed in this report; the Rust handoff only needs the non-spatial UI mapping.

### Stale Docs / Follow-up Docs

The stale Rust implementation wording in this report was updated after the verified Skirmish shell UI sound implementation. Existing docs that call `GenericClick` a generic click sound remain broadly correct, but future wording should say: "`GenericClick` is a distinct `[AudioVisual]` rule field at Rules `+0x70C`; stock YR sets it to `MenuClick`, but Rust should not alias it to `GUIMainButtonSound`."

## Sources

- Ghidra: `VocClass__PlayAtPos @ 0x00750920`; `VocClass__FindByName @ 0x007514D0`; `RulesClass__ReadAudioVisual @ 0x00669300`; `OwnerDraw_Button_00612B70 @ 0x00612B70`; `OwnerDraw_Checkbox_006163A0 @ 0x006163A0`; `OwnerDraw_Trackbar_0061D950 @ 0x0061D950`.
- Prior docs: `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_BUTTON_CLICK_SOUND_PARITY_GHIDRA_REPORT.md`; `C:/Users/enok/Documents/ra2-rust-game-docs/traces/SKIRMISH_CHECKBOX_ICON_VS_LABEL_HIT_TRACE.md`; `C:/Users/enok/Documents/ra2-rust-game-docs/traces/SKIRMISH_CREDITS_TRACKBAR_CLICK_DRAG_TRACE.md`; `C:/Users/enok/Documents/ra2-rust-game-docs/GLOBAL_SOUNDS_GHIDRA_REPORT.md`; `C:/Users/enok/Documents/ra2-rust-game-docs/RULESCLASS_FIELDS.csv`.
- INI: `C:/Users/enok/Documents/ra2-rust-game/ini/rulesmd.ini`; `C:/Users/enok/Documents/ra2-rust-game/ini/rules.ini`; `C:/Users/enok/Documents/ra2-rust-game/ini/soundmd.ini`; `C:/Users/enok/Documents/ra2-rust-game/ini/sound.ini`.
- Rust scan: `C:/Users/enok/Documents/ra2-rust-game/src/app.rs`; `C:/Users/enok/Documents/ra2-rust-game/src/audio/sfx.rs`; `C:/Users/enok/Documents/ra2-rust-game/src/app_transitions.rs`; `C:/Users/enok/Documents/ra2-rust-game/src/rules/ruleset.rs`; `C:/Users/enok/Documents/ra2-rust-game/src/ui/skirmish_shell/state.rs`.
