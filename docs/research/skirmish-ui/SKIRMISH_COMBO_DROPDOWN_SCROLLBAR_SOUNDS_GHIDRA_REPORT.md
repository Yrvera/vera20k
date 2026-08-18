# Skirmish Combo / Dropdown / Scrollbar Sounds - Ghidra Research Report

**Address(es):** `OwnerDraw_ComboBox_00617250 @ 0x00617250`, `ComboDropWin WndProc block @ 0x0060D540`, `OwnerDraw_ScrollBar_0061C690 @ 0x0061C690`, `VocClass__PlayAtPos @ 0x00750920`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** standard YR offline Skirmish `0x102` combo/dropdown sound behavior for collapsed combo arrow open, row selection/close/cancel, dropdown scrollbar up/down/page/thumb actions, and explicit mouse-wheel handling if present. Side combo is the concrete sample; Color/Start/Team/AiType share the owner-draw callback path unless noted.  
**Non-Scope:** dropdown pixel geometry, row text clipping, button/checkbox/trackbar sounds, full combo population semantics, runtime audio capture.  
**Confidence:** High for open/close sound call sites, no scrollbar sound call in the callback, RulesClass source fields, and the post-implementation Rust status. Medium for mouse wheel because no explicit `WM_MOUSEWHEEL` handler was found in the scoped callbacks, but OS/native translation was not runtime-captured.  
**Active in YR:** Yes for owner-draw combo open/close on standard `0x102` combos; Conditional for scrollbar actions only when a dropdown has more rows than visible capacity.

## 0. Working Notes

Target question: Determine whether standard Skirmish combo/dropdown interactions play shell UI sounds or are intentionally silent.  
Non-goals: Do not re-audit dropdown pixel geometry or non-combo shell controls.  
Evidence needed to mark COMPLETE: read prior combo trace/reports, scan INI defaults and Rust combo state, decompile `OwnerDraw_ComboBox_00617250`, inspect `ComboDropWin` row/close assembly around the sound call, decompile `OwnerDraw_ScrollBar_0061C690`, and verify sound source fields/args.  
Stop conditions: every scoped interaction is classified as sound-playing, silent, or explicitly uncertain, with Active in YR stated and a Rust handoff.

## 1. Overview

Standard Skirmish combo controls do play UI sounds, but not for every sub-action. The collapsed combo owner-draw callback plays `[AudioVisual] GUIComboOpenSound` on combo mouse down/double-click before it tests the rightmost 20 px arrow toggle. The `ComboDropWin` popup plays `[AudioVisual] GUIComboCloseSound` when the popup is dismissed or a row-selection path proceeds to close. The owner-draw scrollbar callback is silent: arrow/page/thumb changes send a parent scroll notification and invalidate the scrollbar, but do not call `VocClass__PlayAtPos`.

Current Rust now has the combo/dropdown state path, parsed `gui_combo_open_sound` / `gui_combo_close_sound` fields, Skirmish combo sound events, and app playback wired. The player-visible contract remains: opening a side/color/start/team/AI dropdown and selecting/canceling it should produce the RA2/YR shell combo open/close sounds, while scrollbar-only nudges should not add extra sounds.

## 2. Key Fields / Sound Sources

| Field / key | Behavior | Evidence | Active in YR |
|---|---|---|---|
| Rules `+0x1A4` / `[AudioVisual] GUIComboOpenSound` | Sound index used by combo mouse down/double-click path | assembly `0x006184A2..0x006184BA`; `RULESCLASS_FIELDS.csv`; `GLOBAL_SOUNDS_GHIDRA_REPORT.md`; `rulesmd.ini` default `MenuACBOpen` | Yes |
| Rules `+0x1A8` / `[AudioVisual] GUIComboCloseSound` | Sound index used by `ComboDropWin` close/selection path | assembly `0x0060E4E9..0x0060E500`; `RULESCLASS_FIELDS.csv`; `GLOBAL_SOUNDS_GHIDRA_REPORT.md`; `rulesmd.ini` default `MenuACBClose` | Yes |
| `VocClass__PlayAtPos` args | `ECX = sound index`, `EDX = 0x2000`, stack volume `1.0f`, stack handle/source `0` | decompile `0x00750920`; call sites `0x006184A8..0x006184BA`, `0x0060E4EE..0x0060E500` | Yes |
| Owner-draw scrollbar state | Range/current/thumb/pressed state; no sound field is read | decompile `OwnerDraw_ScrollBar_0061C690`; no `VocClass__PlayAtPos` call in callback | Conditional when dropdown has scrollbar |
| `ComboDropWin` class | Popup WndProc block `0x0060D540` owns popup close/select behavior | `FUN_0060D450 @ 0x0060D450` registers `"ComboDropWin"` with `LAB_0060D540` | Yes |

## 3. Core Logic

### 3.1 Collapsed Combo Open Sound

Active in YR: Yes. `OwnerDraw_ComboBox_00617250` handles `WM_LBUTTONDOWN (0x201)` and `WM_LBUTTONDBLCLK (0x203)` through the same branch. The first side effect in that branch is a `VocClass__PlayAtPos` call using Rules `+0x1A4`, volume `1.0`, `EDX = 0x2000`, and handle/source `0`. Only after the sound call does it compare mouse X against `client_width - 0x14`; if the click is in the rightmost 20 px, it reads `CB_GETDROPPEDSTATE (0x157)` and posts `CB_SHOWDROPDOWN (0x14F)` with the inverted open state.

Evidence: decompile `OwnerDraw_ComboBox_00617250`; assembly `0x006184A2..0x00618507`. The sound call block is:

- `0x006184A2`: load global Rules pointer `0x008871E0`
- `0x006184A8`: push handle/source `0`
- `0x006184AA`: set `EDX = 0x2000`
- `0x006184AF`: push `0x3F800000` (`1.0f`)
- `0x006184B4`: load `Rules + 0x1A4`
- `0x006184BA`: call `VocClass__PlayAtPos`
- `0x006184CD..0x00618507`: rightmost-20-px gate and `PostMessageA(..., 0x14F, inverse_dropped_state, 0)`

This callback is shared by the standard Skirmish combo families because `FUN_0060F9A0` subclasses `"ComboBox"` controls to `OwnerDraw_ComboBox_00617250`, and prior combo reports verify Side/Color/Start/Team/AiType controls are standard `0x102` combo paths.

### 3.2 Dropdown Row Selection / Close / Cancel Sound

Active in YR: Yes. The `ComboDropWin` WndProc block at `0x0060D540` contains the popup close/select sound. The sound call at `0x0060E500` uses Rules `+0x1A8` (`GUIComboCloseSound`), volume `1.0`, `EDX = 0x2000`, and handle/source `0`. The row-selection path then checks whether the click coordinates are within the popup content bounds, computes row data through source-combo messages (`CB_GETITEMHEIGHT 0x154`, `CB_GETCOUNT 0x146`, item lookup), sends selection/follow-up messages, and closes the dropdown with `CB_SHOWDROPDOWN 0x14F, wParam=0`.

Evidence: assembly `0x0060E4A0..0x0060E616`. The close sound is at `0x0060E4E9..0x0060E500`; the following bounds checks at `0x0060E505..0x0060E523` decide whether a row-selection sequence follows or the path closes/cancels. The close call at `0x0060E606..0x0060E616` sends `0x14F` with `wParam=0` to the source combo. The in-content branch at `0x0060E5AA..0x0060E5D9` sends the selection/parent notification sequence before return.

The important player-visible distinction: the sound is `GUIComboCloseSound`, not another `GUIComboOpenSound`, not `GUIMainButtonSound`, and not `GenericClick`.

### 3.3 Scrollbar Actions Are Silent

Active in YR: Conditional. Side/country dropdowns normally exceed the visible-row cap and can create a scrollbar child. `OwnerDraw_ScrollBar_0061C690` handles `WM_LBUTTONDOWN`, `WM_LBUTTONUP`, `WM_MOUSEMOVE`, `WM_TIMER`, range/current messages, grey state, and paint. It mutates current/top value, pressed arrow/thumb state, timers, capture, and invalidation. On value change, it sends parent `WM_VSCROLL (0x115)` with `(current << 16) | scroll_code` and invalidates the scrollbar.

No `VocClass__PlayAtPos` call, Rules `+0x1A4`, Rules `+0x1A8`, or other sound source read is present in the decompiled scrollbar callback. Therefore scrollbar up/down arrow repeat, page-click, and thumb drag are intentionally silent at the scrollbar callback level.

Evidence: decompile `OwnerDraw_ScrollBar_0061C690`; value-change send at final branch `SendMessageA((HWND)piVar11[2], 0x115, uVar4 << 16 | uStack_e4, param_1)`; paint/input branches `0x0061D383` and related assembly contexts show timers/capture/invalidates but no `0x00750920` call.

### 3.4 Mouse Wheel

Active in YR: No explicit scoped handler found. `OwnerDraw_ScrollBar_0061C690` does not handle `WM_MOUSEWHEEL (0x20A)`. A read-only assembly pass across the material `ComboDropWin` switch blocks did not find a `0x20A` handler or a sound call tied to wheel input. If the retail window receives wheel input through an OS/native translation path, that specific runtime behavior was not captured in this slot.

Evidence: decompile `OwnerDraw_ScrollBar_0061C690`; `ComboDropWin` assembly contexts around `0x0060D540..0x0060F307`; no `0x20A` branch found in the scoped pass.

## 4. INI Keys

| INI key | Default | Rules offset | Scoped effect | Active in YR |
|---|---|---:|---|---|
| `[AudioVisual] GUIComboOpenSound` | `MenuACBOpen` | `+0x1A4` | collapsed combo mouse-down/double-click sound | Yes |
| `[AudioVisual] GUIComboCloseSound` | `MenuACBClose` | `+0x1A8` | popup close/select/cancel sound | Yes |
| `[AudioVisual] GUIMainButtonSound` | `MenuClick` | `+0x188` | not used by combo/dropdown paths in this slice | Yes elsewhere, No for scoped combo |
| `[AudioVisual] GenericClick` | `MenuClick` | `+0x70C` | not used by combo/dropdown paths in this slice | Yes elsewhere, No for scoped combo |

Evidence: `ini/rulesmd.ini` `[AudioVisual]` block; `GLOBAL_SOUNDS_GHIDRA_REPORT.md`; `RULESCLASS_FIELDS.csv`; call-site offsets above.

## 5. Integration Points

| Function / block | Role | Sound verdict | Active in YR |
|---|---|---|---|
| `FUN_0060F9A0` | subclasses standard shell `"ComboBox"` and `"Scrollbar"` controls into owner-draw callbacks | no direct scoped sound | Yes |
| `OwnerDraw_ComboBox_00617250` | collapsed combo input/open/paint and source combo wrapper | plays `GUIComboOpenSound` on mouse down/double-click | Yes |
| `ComboDropWin @ 0x0060D540` | popup row, select, close, cancel behavior | plays `GUIComboCloseSound` on close/select/cancel path | Yes |
| `OwnerDraw_ScrollBar_0061C690` | popup scrollbar input/paint/range/current state | silent; sends scroll notification only | Conditional |
| `VocClass__PlayAtPos @ 0x00750920` | sound playback helper | resolves sound index and bails out if disabled/out-of-range/missing | Yes |

## 6. Current Rust Implementation Status

Rust now has the previously missing combo sound behavior:

- `src/ui/skirmish_shell/state.rs` implements `combo_arrow_at`, `open_combo_dropdown`, `scroll_open_combo_by_rows`, `apply_combo_selection`, dropdown close/selection state transitions, and Skirmish combo sound events for the open/close paths.
- `src/app.rs` routes Skirmish mouse input through the Skirmish shell path and plays the emitted combo open/close sound events through app-level shell audio playback.
- `src/rules/ruleset.rs` / `RuleSet::General` now expose `gui_combo_open_sound` and `gui_combo_close_sound` in addition to `gui_main_button_sound`, so the Skirmish shell can use the same `[AudioVisual]` sound keys verified in `gamemd.exe`.
- Scrollbar-only dropdown actions remain silent, matching the verified `OwnerDraw_ScrollBar_0061C690` sound contract.

Verify-doc slot 5 on 2026-05-22 audited 22 claims as YELLOW: 18 confirmed, 0 wrong, 3 stale implementation-status claims, and 1 unverifiable mouse-wheel delivery caveat. This cleanup resolves the stale current-Rust wording without changing the core binary claims.

No Rust files were modified by this documentation cleanup.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Prior trace/doc scan | verified | `SKIRMISH_SIDE_COMBO_DROPDOWN_OPEN_SELECT_SCROLL_TRACE.md`; combo visual/owner-draw reports | none |
| INI defaults and Rules offsets | verified | `rulesmd.ini`; `GLOBAL_SOUNDS_GHIDRA_REPORT.md`; `RULESCLASS_FIELDS.csv` | none |
| Combo open mouse branch | verified | `0x006184A2..0x00618507`; decompile `0x00617250` | runtime audibility of non-arrow face clicks if desired |
| Dropdown close/select/cancel sound | verified | `0x0060E4E9..0x0060E616` | exact all-message taxonomy of popup WndProc remains not reconstructed as one decompiled function |
| Scrollbar arrow/page/thumb sound | verified | decompile `0x0061C690`; no sound call; parent `0x115` send | none for scoped sound claim |
| Mouse wheel | touched-not-exhausted | no `0x20A` in scoped decompile/assembly pass | runtime capture or wider Win32 dispatch audit if wheel parity matters |
| Current Rust status after sound implementation | verified | `src/ui/skirmish_shell/state.rs`; `src/app.rs`; `src/rules/ruleset.rs` scans; verify-doc slot 5 on 2026-05-22 | stale doc wording was updated; runtime mouse-wheel delivery remains unverifiable |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-SK-COMBO-SND-001 - Does opening a Skirmish combo play a sound? -> Yes, `GUIComboOpenSound` from Rules `+0x1A4`, default `MenuACBOpen`.` (evidence: `0x006184A2..0x006184BA`; `rulesmd.ini`; `RULESCLASS_FIELDS.csv`)
- `[RESOLVED] OQ-SK-COMBO-SND-002 - Does selecting/closing/canceling a popup play a sound? -> Yes, `GUIComboCloseSound` from Rules `+0x1A8`, default `MenuACBClose`, on the `ComboDropWin` close/select/cancel path.` (evidence: `0x0060E4E9..0x0060E616`; `rulesmd.ini`; `RULESCLASS_FIELDS.csv`)
- `[RESOLVED] OQ-SK-COMBO-SND-003 - Are scrollbar arrow/page/thumb actions separately sounded? -> No; the scrollbar callback sends `WM_VSCROLL` and invalidates but does not call sound playback.` (evidence: `OwnerDraw_ScrollBar_0061C690 @ 0x0061C690`)
- `[RESOLVED] OQ-SK-COMBO-SND-004 - Do Side/Color/Start/Team/AiType share the callback? -> Yes for standard `0x102` owner-draw combos subclassed by `FUN_0060F9A0`; combo reports verify these families consume the shared owner-draw callback.` (evidence: prior `SKIRMISH_OWNERDRAW_COMBOBOX_00617250_GHIDRA_REPORT.md`; `FUN_0060F9A0`)
- `[RESOLVED] OQ-SK-COMBO-SND-005 - Do combo/dropdown paths use button sounds or `GenericClick`? -> No for this slice; open uses `+0x1A4`, close uses `+0x1A8`.` (evidence: scoped call sites; button sound report covers separate `+0x188`/`+0x70C` paths)
- `[RESOLVED] OQ-SK-COMBO-SND-006 - Does current Rust already parse/play the needed combo sound keys? -> Yes after the verified Skirmish shell UI sound implementation; `gui_combo_open_sound` and `gui_combo_close_sound` are now available to Skirmish combo sound events and app playback.` (evidence: `src/rules/ruleset.rs`; `src/app.rs`; verify-doc slot 5 on 2026-05-22)
- `[DEFERRED] OQ-SK-COMBO-SND-007 - Does retail mouse wheel scroll a dropdown through an unscoped OS translation path?` (category: `needs-runtime-debugger`; reason: no explicit `0x20A` handler found in scoped callbacks, but runtime input delivery was not captured; next-step-if-pursued: live retail wheel input trace)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust status | Affected Rust surface | Implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Combo mouse down/double-click plays `GUIComboOpenSound` (`MenuACBOpen` by default) with `VocClass__PlayAtPos(sound,+0x2000,1.0,0)` before posting dropdown toggle | `0x006184A2..0x00618507`; Rules `+0x1A4` | implemented: key is parsed/stored and app playback is wired through Skirmish combo open sound events | `src/rules/ruleset.rs`, `src/app.rs`, `src/ui/skirmish_shell/state.rs` | Preserve the implemented `GUIComboOpenSound` plumbing and event emission from Skirmish combo mouse-down handling when an enabled combo arrow/open attempt is processed | Click local Side combo arrow in native shell; one `MenuACBOpen` SFX is requested before dropdown state becomes open | Do not reuse `GUIMainButtonSound`; regression test target `skirmish_combo_arrow_open_plays_gui_combo_open_sound` |
| Popup close/select/cancel path plays `GUIComboCloseSound` (`MenuACBClose` by default) before final selection/close route | `0x0060E4E9..0x0060E616`; Rules `+0x1A8` | implemented: key is parsed/stored and app playback is wired through Skirmish combo close sound events | `src/rules/ruleset.rs`, `src/app.rs`, `src/ui/skirmish_shell/state.rs` | Preserve the implemented `GUIComboCloseSound` plumbing and close-event emission when an open dropdown is dismissed by row selection or outside cancel | Open Side combo, click `Great Britain`; one `MenuACBClose` SFX is requested and selected country updates | Do not play close sound for scrollbar-only clicks; regression test target `skirmish_dropdown_row_selection_plays_gui_combo_close_sound_once` |
| Scrollbar actions are silent and only produce scroll state/parent notification | decompile `OwnerDraw_ScrollBar_0061C690`; no sound call; `WM_VSCROLL 0x115` send | current silence matches sound contract | `src/ui/skirmish_shell/state.rs`, `src/app.rs` | Preserve silence for dropdown scrollbar arrow/page/thumb changes while adding open/close sounds around combo/dropdown actions | Open Side combo, click scrollbar down arrow; top index changes and no SFX is requested | Do not attach `GUIComboOpenSound` or `GUIComboCloseSound` to each scroll nudge; proposed test `skirmish_dropdown_scrollbar_clicks_do_not_play_combo_sounds` |

## Negative Facts / Do Not Do

- Do not use `GUIMainButtonSound` for combo open/close. Active in YR: No for scoped combo paths. Evidence: combo open uses Rules `+0x1A4`, close uses `+0x1A8`; button report separately verifies `+0x188`.
- Do not use `GenericClick` for combo/dropdown interactions. Active in YR: No for scoped combo paths. Evidence: no `+0x70C` read at the scoped combo/dropdown call sites.
- Do not play sounds for dropdown scrollbar arrow/page/thumb clicks. Active in YR: No. Evidence: `OwnerDraw_ScrollBar_0061C690` has no `VocClass__PlayAtPos` call and only sends `WM_VSCROLL`.
- Do not invent a mouse-wheel sound. Active in YR: No explicit scoped evidence. Evidence: no `WM_MOUSEWHEEL 0x20A` handler or sound call found in scoped callbacks.
- Do not implement combo sounds in `sim/`. Active in YR: this is shell UI/audio behavior. Evidence: all verified calls live in owner-draw window procs and Rules UI sound fields.

## Remaining Uncertainty

- Runtime mouse-wheel delivery remains uncertain: no explicit handler was found, but this slot did not use a runtime debugger to prove whether Windows translates wheel input into another message path for the popup.
- Exact audible behavior when clicking the non-arrow face of a collapsed combo is a follow-up edge case if desired; binary order shows the sound precedes the rightmost-20-px gate, but the scoped acceptance target is arrow-open.

## Stale Docs / Follow-up Docs

- None found requiring replacement. Prior combo visual reports did not settle this sound behavior.

## Sources

- Ghidra read-only decompile: `OwnerDraw_ComboBox_00617250 @ 0x00617250`, `OwnerDraw_ScrollBar_0061C690 @ 0x0061C690`, `FUN_0060D450 @ 0x0060D450`, `VocClass__PlayAtPos @ 0x00750920`.
- Ghidra read-only assembly contexts: `0x006184A2..0x00618507`, `0x0060E4A0..0x0060E616`, `0x0061D383` input block and final `WM_VSCROLL` send in `0x0061C690`.
- Docs: `GLOBAL_SOUNDS_GHIDRA_REPORT.md`, `RULESCLASS_FIELDS.csv`, `SKIRMISH_OWNERDRAW_COMBOBOX_00617250_GHIDRA_REPORT.md`, `SKIRMISH_COMBO_DROPDOWN_VISUAL_PARITY_GHIDRA_REPORT.md`, `traces/SKIRMISH_SIDE_COMBO_DROPDOWN_OPEN_SELECT_SCROLL_TRACE.md`.
- INI: `ini/rulesmd.ini`, `ini/rules.ini`.
- Rust scanned: `src/ui/skirmish_shell/state.rs`, `src/app.rs`, `src/rules/ruleset.rs`.
