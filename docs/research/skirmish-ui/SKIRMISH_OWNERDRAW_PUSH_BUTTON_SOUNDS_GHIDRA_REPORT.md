# Skirmish Owner-Draw Push Button Sounds - Ghidra Research Report

**Address(es):** `0x00612B70`, `0x0060F9A0`, `0x005E68A0`, callback entry `0x005E6920`, `0x00750920`  
**Investigation Mode:** exhaustive-slice  
**Target question:** Which Skirmish shell owner-draw push buttons play which retail UI sounds, when do those calls happen, and what does Rust need to reproduce for dialog `0x102` and modal `0x6B`?  
**Non-goals:** button art geometry, text rendering, Choose Map modal list/preview behavior, Start validation semantics beyond disabled/suppressed sound effects, and checkbox/trackbar/combo sounds.  
**Evidence needed to mark COMPLETE:** binary spot-checks for the shared owner-draw callback, modal `0x6B` button routing, sound INI/default source fields, suppressed/disabled branches, command-ordering boundary, current Rust sound surfaces, implementation handoff, and negative facts.  
**Stop conditions:** write only this report plus the shared swarm claims file, keep Ghidra read-only, and leave all Rust/INI/in-repo docs untouched.  
**Claimed Scope:** dialog `0x102` Start Game `0x617`, Choose Map `0x5AA`, Back `0x5C0`; modal `0x6B` Use Map `0x6C5`, Create Random Map `0x583`, Cancel `0x5C0` only as far as those controls use the same `OwnerDraw_Button_00612B70` sound contract.  
**Confidence:** High for shared callback routing, two sound call sites, INI source fields/defaults, call ordering before command dispatch, and current Rust implementation status after verify-doc slot 2 cleanup on 2026-05-22. Medium for whether both binary sound call sites are always audibly distinct at runtime because message coalescing needs live capture.  
**Active in YR:** Yes for standard offline Skirmish setup and Choose Map modal; Conditional for Create Random Map because the branch requires the player to click that modal button.

## 1. Overview

Retail shell push-button audio is not emitted by the Skirmish action handlers. The sound sites live in the shared owner-draw button subclass: `WM_LBUTTONDOWN`/`WM_LBUTTONDBLCLK` plays `[AudioVisual] GUIMainButtonSound`, and the first enabled paint transition from released to pressed can play `[AudioVisual] GenericClick`.

The modal Choose Map buttons are not a separate sound family. Dialog `0x6B` creates normal `BUTTON` controls with style `0x5000000B`; the common shell setup routes those through the same `OwnerDraw_Button_00612B70` callback as the `0x102` Start/Choose/Back buttons.

## 2. Key Offsets / Fields

| Field / global | Meaning in this slice | Evidence | Active in YR |
|---|---|---|---|
| `RulesClass + 0x188` | `[AudioVisual] GUIMainButtonSound`, stock `MenuClick`; used on owner-draw mouse down/double-click | `0x00613759..0x00613771`; `ini/rulesmd.ini:643` | Yes |
| `RulesClass + 0x70C` | `[AudioVisual] GenericClick`, stock `MenuClick`; used on first enabled paint transition from `'u'` to `'d'` | `0x00613273..0x00613289`; `ini/rulesmd.ini:703` | Yes |
| owner record `+0xBC` / `piVar17[0x2F]` low byte | suppress/blocked byte; nonzero returns before mouse sound and before paint work | `0x0061374B..0x00613753`; `OwnerDraw_Button_00612B70` paint early-out | Conditional |
| owner record `+0xC5` | timer/hover-like visual byte toggled by `WM_TIMER`, not a sound source selector in the default PCX button path | `OwnerDraw_Button_00612B70` decompile around `WM_TIMER`; prior pixel-layout report | Yes, but not a sound gate for this slice |
| `DAT_00833684` | global last-rendered owner-draw PCX button state char; `GenericClick` requires previous `'u'` and current `'d'` | `0x00613264..0x0061329B` | Yes |
| `WS_DISABLED` style `0x08000000` | suppresses paint-transition `GenericClick` by forcing state back to `'u'`; disabled alpha overlay belongs to visual path | `0x00613254..0x00613262`; prior chrome-control report | Conditional |

## 3. Core Logic

### 3.1 Shared Callback Selection

`FUN_0060F9A0` classifies child controls by class name and low style bits. For class `"Button"`, it checks `(style & 7) == 7` first, then `(style & 0x0B) == 0x0B`; the second branch installs `OwnerDraw_Button_00612B70`.

Active in YR: Yes. Evidence: Ghidra decompile of `FUN_0060F9A0`; assembly context `0x0060FE78..0x0060FE8B` shows `AND EDX,0xB`, `CMP DL,0xB`, then `MOV EBP,0x612B70`.

Dialog `0x102` buttons `0x617`, `0x5AA`, and `0x5C0` were already verified as this style in sibling reports. Dialog `0x6B` buttons `0x6C5`, `0x583`, and `0x5C0` are also `BUTTON` controls with style `0x5000000B`, so they satisfy the same `(style & 0x0B) == 0x0B` branch.

Active in YR: Yes for `0x617`, `0x5AA`, `0x5C0`, `0x6C5`; Conditional for `0x583` by player action. Evidence: `SKIRMISH_CHOOSE_MAP_MODAL_VISUAL_CONTROL_LAYOUT_GHIDRA_REPORT.md` resource table and `0x005E69C2..0x005E69EC` command dispatch. Verify-doc slot 2 did not independently re-extract the modal resource style, so the `0x5000000B` modal-button evidence remains inherited from the sibling modal layout report.

### 3.2 Mouse-Down / Double-Click Sound

In `OwnerDraw_Button_00612B70`, messages `0x201` and `0x203` share one active sound path:

1. Read owner record byte `+0xBC`.
2. If nonzero, return `0` immediately; no sound and no default Button proc continuation from this branch.
3. Load global Rules pointer from `0x008871E0`.
4. Push sound handle/source argument `0`.
5. Set `EDX = 0x2000`.
6. Push volume `1.0f` (`0x3F800000`).
7. Load `ECX = [RulesClass + 0x188]`.
8. Call `VocClass__PlayAtPos @ 0x00750920`.
9. Continue into the previous Button window proc so capture/pressed state and later command dispatch proceed normally.

Active in YR: Yes, unless the per-control suppress byte is set. Evidence: decompile of `OwnerDraw_Button_00612B70`; assembly context `0x0061374B..0x00613776`.

### 3.3 Paint-Transition Sound

In `WM_PAINT`, the same callback derives a state char:

- starts from `'u'`;
- if the standard Button pressed bit is set, becomes `'d'`;
- if `WS_DISABLED` is set, the state is forced back to `'u'`;
- if not disabled, current state is `'d'`, and `DAT_00833684` is `'u'`, it plays `RulesClass + 0x70C` through `VocClass__PlayAtPos`;
- then it stores the current state char into `DAT_00833684`.

The `GenericClick` call uses the same argument shape as the mouse-down call: handle/source `0`, `EDX = 0x2000`, volume `1.0f`, sound id in `ECX`.

Active in YR: Yes, conditional on an unsuppressed paint, enabled style, current pressed state, and global previous-state byte `'u'`. Evidence: assembly context `0x0061323C..0x0061329B`.

### 3.4 Command Dispatch Order

The owner-draw sound calls happen before Skirmish setup or modal command handlers execute. For `0x102`, `FUN_006AE3F0` consumes `WM_COMMAND` later and calls `FUN_006ACEE0`. For `0x6B`, the callback entry at `0x005E6920` handles `WM_COMMAND` later, splitting the low control id and branching on `0x6C5`, `0x583`, and `0x5C0`.

Active in YR: Yes. Evidence: prior `0x102` report for `0x006AE425..0x006AE448`; fresh assembly context `0x005E69C2..0x005E69EC` for modal branch ids; `0x005E68A0` creates dialog `0x6B` with callback entry `0x005E6920`.

### 3.5 Sound Playback Helper Semantics

`VocClass__PlayAtPos @ 0x00750920` treats `ECX`/`param_1` as the sound index. It returns early when the global audio-ready byte is zero, rejects negative or out-of-range ids, resolves the sound object from the global sound table, allocates/reuses a sound event, then sets volume and pan.

Active in YR: Yes. Evidence: decompile of `0x00750920`; direct calls from `0x00613771` and `0x00613289`.

## 4. INI Keys

| INI key | Stock YR value | Rules offset | Effect here | Active in YR |
|---|---|---:|---|---|
| `[AudioVisual] GUIMainButtonSound` | `MenuClick` | `+0x188` | Mouse-down / double-click owner-draw push-button sound | Yes |
| `[AudioVisual] GenericClick` | `MenuClick` | `+0x70C` | Paint-time released-to-pressed transition sound | Yes |
| `[AudioVisual] ShellButtonSlideSound` | empty in stock YR | `+0x750` per sibling report | Not used by these click paths | No for this slice |

Base RA2 has the same relevant defaults: `ini/rules.ini` contains `GUIMainButtonSound=MenuClick` and `GenericClick=MenuClick`. YR `rulesmd.ini` is still the priority source.

## 5. Integration Points

| Integration | Behavior | Evidence | Active in YR |
|---|---|---|---|
| Setup dialog `0x102` | Start `0x617`, Choose Map `0x5AA`, Back `0x5C0` use shared owner-draw button callback; command actions run after child Button processing | `SKIRMISH_BUTTON_CLICK_SOUND_PARITY_GHIDRA_REPORT.md`; `0x00612B70` spot-check | Yes |
| Choose Map modal `0x6B` | Wrapper creates dialog `0x6B`, passes callback entry `0x005E6920`, runs common owner-draw shell setup, then shows and pumps modal | `0x005E68B7..0x005E690F`; decompile `FUN_005e68a0` | Yes |
| Modal right-column buttons | `0x6C5` Use Map, `0x583` Create Random Map, `0x5C0` Cancel are style `0x5000000B` Buttons, so use `OwnerDraw_Button_00612B70` | resource layout doc; `0x0060FE78..0x0060FE8B`; `0x005E69C2..0x005E69EC` | Yes / Conditional for `0x583` |
| Sound playback | Owner-draw passes rules-resolved sound id to `VocClass__PlayAtPos`; empty/out-of-range audio ids naturally no-op in `0x00750920` | `0x00750920` decompile | Yes |

## 6. Current Rust Implementation Status

| Rust area | Status | Evidence |
|---|---|---|
| `GUIMainButtonSound` parser | implemented and used by shell button press paths | verify-doc slot 2 cleanup, 2026-05-22 |
| `GenericClick` parser | implemented | verify-doc slot 2 cleanup, 2026-05-22 |
| main-menu button sound | implemented helper uses `rules.general.gui_main_button_sound` | prior current-Rust scan |
| Skirmish `0x102` mouse down | implemented; Skirmish shell button press emits `GUIMainButtonSound` before release/action handling | verify-doc slot 2 cleanup, 2026-05-22 |
| Skirmish `0x102` paint transition | implemented; `skirmish_shell_last_painted_pressed_button` tracks the pressed-paint transition for `GenericClick` | verify-doc slot 2 cleanup, 2026-05-22 |
| Skirmish `0x102` mouse up/action | release-inside action gate remains separate from press sound timing | prior current-Rust scan |
| Skirmish modal `0x6B` button sounds | not reachable because the modal itself is not integrated | `src/app.rs:586`; prior modal integration docs |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `OwnerDraw_Button_00612B70` mouse `0x201/0x203` sound | verified | `0x0061374B..0x00613776` | none |
| `OwnerDraw_Button_00612B70` paint `GenericClick` branch | verified | `0x0061323C..0x0061329B` | runtime audible double-click/double-sound capture if exact audibility matters |
| Disabled style suppressing `GenericClick` | verified | `0x00613254..0x00613262`; chrome-control sibling report | disabled Win32 event delivery not separately runtime-captured |
| owner byte `+0xBC` suppressing sound/paint | verified for this callback | `0x0061374B..0x00613753`; decompile paint early-out | all writers to `+0xBC` out of scope |
| `FUN_0060F9A0` Button style routing | verified | `0x0060FE78..0x0060FE8B` | none |
| `0x102` Start/Choose/Back active path | verified by sibling docs and spot-check | `SKIRMISH_BUTTON_CLICK_SOUND_PARITY_GHIDRA_REPORT.md`; `OwnerDraw_Button_00612B70` | none |
| `0x6B` modal wrapper and callback entry | verified | `0x005E68B7..0x005E690F` | no full callback decompile because Ghidra lacks function boundary at `0x005E6920` |
| `0x6B` button resource styles and dispatch ids | verified | modal layout doc; `0x005E69C2..0x005E69EC` | random-map downstream out of scope |
| `VocClass__PlayAtPos` sound id behavior | verified | `0x00750920` decompile | none |
| Verify-doc slot 2 claim audit | YELLOW: 22 claims audited; 17 confirmed, 0 wrong, 4 stale, 1 unverifiable | 2026-05-22 verify-doc slot 2 | stale current-Rust wording patched here |
| Current Rust Skirmish sound parity | implemented for setup `0x102` press sound and paint-transition tracking | verify-doc slot 2 cleanup, 2026-05-22 | modal `0x6B` reuse when the live modal route is integrated |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-1 - Do modal 0x6B buttons use the same owner-draw callback as setup 0x102 buttons? -> Yes; modal buttons are style 0x5000000B BUTTON controls and FUN_0060F9A0 maps (style & 0x0B)==0x0B to OwnerDraw_Button_00612B70.` (evidence: modal resource doc; `0x0060FE78..0x0060FE8B`)
- `[RESOLVED] OQ-2 - Which sound plays on mouse down/double-click? -> Rules +0x188, [AudioVisual] GUIMainButtonSound, stock MenuClick.` (evidence: `0x00613759..0x00613771`; `ini/rulesmd.ini:643`)
- `[RESOLVED] OQ-3 - Which sound plays on paint transition? -> Rules +0x70C, [AudioVisual] GenericClick, stock MenuClick, only when enabled current state is 'd' and DAT_00833684 was 'u'.` (evidence: `0x00613264..0x00613289`; `ini/rulesmd.ini:703`)
- `[RESOLVED] OQ-4 - Does command dispatch play the button sounds? -> No; sounds happen in child owner-draw callback before default Button processing produces parent WM_COMMAND.` (evidence: `0x00613771` before CallWindowProc continuation; `0x005E69C2..0x005E69EC`)
- `[RESOLVED] OQ-5 - What suppresses button sounds? -> Owner record +0xBC suppresses mouse sound and paint; WS_DISABLED suppresses the paint-transition GenericClick by forcing released state.` (evidence: `0x0061374B..0x00613753`; `0x00613254..0x00613262`)
- `[RESOLVED] OQ-6 - Is ShellButtonSlideSound part of this path? -> No; this callback loads only +0x188 and +0x70C, while ShellButtonSlideSound is documented at a separate shell slide-in site.` (evidence: `OwnerDraw_Button_00612B70` decompile; sibling slide-sound report)
- `[RESOLVED] OQ-7 - Does current Rust parse and play both sounds for Skirmish buttons? -> Yes for the implemented setup `0x102` Skirmish shell path: Rust parses `GUIMainButtonSound` and `GenericClick`, plays the Skirmish shell button press sound, and tracks `skirmish_shell_last_painted_pressed_button` for the `GenericClick` paint transition. Modal `0x6B` reuse remains tied to future live modal-route integration.` (evidence: verify-doc slot 2 cleanup, 2026-05-22)
- `[DEFERRED] OQ-8 - Are both mouse-down and paint-transition calls audibly distinct on every retail click?` (category: `needs-runtime-debugger`; reason: binary proves both call sites and conditions, but Windows paint/message timing can coalesce the second audible event; next-step-if-pursued: runtime trace/capture one held click over Start and one modal Use Map click)
- `[DEFERRED] OQ-9 - Which helper(s) set owner record +0xBC for every shell button?` (category: `out-of-scope`; reason: this slot only needed the effect of +0xBC on push-button sound/paint; next-step-if-pursued: trace common subclass messages that write `piVar17[0x2F]`)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Owner-draw push buttons play `GUIMainButtonSound` on mouse down/double-click before command dispatch, unless the control suppress byte blocks the callback. | `0x0061374B..0x00613776`; `ini/rulesmd.ini:643` | implemented for Skirmish setup `0x102` | `src/app.rs`, `src/ui/skirmish_shell/state.rs`, `src/rules/ruleset.rs` | Maintain Skirmish shell button mouse-down sound using the parsed `gui_main_button_sound`, before release/action handling. | Press Start, Choose Map, or Back in native Skirmish shell: one `MenuClick` request is emitted on press even if release later drags off; command action still waits for release-inside. Regression test: `skirmish_ownerdraw_button_mouse_down_plays_gui_main_button_sound_before_action`. | Do not play this only on mouse-up/action; that shifts sound after retail. |
| Enabled owner-draw push-button paint can play `GenericClick` on first global `'u' -> 'd'` transition. | `0x00613264..0x00613289`; `ini/rulesmd.ini:703` | implemented for Skirmish setup `0x102` via parsed `GenericClick` and `skirmish_shell_last_painted_pressed_button` tracking | `src/rules/ruleset.rs`, `src/app_skirmish_shell_render.rs` or app-layer render state | Maintain `[AudioVisual] GenericClick` as a render/paint-state side effect, not an action side effect. | Hold Start after an unpressed frame: renderer state transition can emit `GenericClick`; disabled Start does not. Regression test: `skirmish_ownerdraw_button_pressed_paint_transition_plays_generic_click_once`. | Do not use `GUIMainButtonSound` for the paint-transition site unless `GenericClick` resolves to the same INI value. |
| Modal `0x6B` Use Map, Cancel, and Create Random Map buttons share the same owner-draw sound contract. | resource style `0x5000000B` from sibling modal layout report; `0x0060FE78..0x0060FE8B`; `0x005E69C2..0x005E69EC` | no live modal route yet | `src/app.rs`, `src/ui/skirmish_shell/state.rs`, future `0x6B` render/input surfaces | When Choose Map modal is integrated, route its push buttons through the same button-sound helper and transition tracking as setup `0x102`. | Open Choose Map, press Use Map or Cancel: press sound occurs before accept/cancel modal result; Create Random Map does the same when clicked. Proposed test: `choose_map_modal_buttons_reuse_skirmish_ownerdraw_button_sound_contract`. | Do not create modal-specific button sound keys or bespoke button audio. |

## 10. Negative Facts / Do Not Do

- Do not emit Skirmish push-button sound from `FUN_006ACEE0`-equivalent action code. Active in YR: No; evidence shows sound in child callback at `0x00613771` before parent command dispatch.
- Do not use `[AudioVisual] ShellButtonSlideSound` for Start/Choose/Back or modal button clicks. Active in YR: No for this path; evidence: `OwnerDraw_Button_00612B70` loads only Rules `+0x188` and `+0x70C`.
- Do not treat Choose Map modal buttons as a separate art/audio family. Active in YR: No; evidence: `0x6B` resource style `0x5000000B` and `FUN_0060F9A0` maps the same style to `OwnerDraw_Button_00612B70`.
- Do not play `GenericClick` from mouse-down logic. Active in YR: No; evidence: `GenericClick` is loaded only in the `WM_PAINT` transition block at `0x00613273..0x00613289`.
- Do not ignore disabled/suppressed cases. Active in YR: Conditional; evidence: owner byte `+0xBC` suppresses mouse sound/paint, and `WS_DISABLED` prevents the paint-transition `GenericClick`.

## 11. Stale Docs / Follow-up Docs

No stale-doc replacement wording found. This report extends `docs/research/skirmish-ui/SKIRMISH_BUTTON_CLICK_SOUND_PARITY_GHIDRA_REPORT.md` by confirming modal `0x6B` reuse and by spelling out Rust test surfaces; it does not contradict that report.

## 12. Remaining Uncertainty

- Runtime capture is still needed to say whether the mouse-down `GUIMainButtonSound` and paint-transition `GenericClick` are always heard as two separate clicks on a normal held press.
- The full writer set for owner record `+0xBC` was not traced; this report only verifies the byte's effect when already set.
- Ghidra did not expose a clean function boundary at callback entry `0x005E6920`; modal command ids were verified from assembly context and sibling resource/flow reports instead.
- Verify-doc slot 2 did not independently re-extract modal resource style `0x5000000B`; this report continues to rely on the sibling modal layout report for that resource fact.

## Sources

- Fresh read-only Ghidra decompile / assembly context: `OwnerDraw_Button_00612B70 @ 0x00612B70`, `FUN_0060F9A0 @ 0x0060F9A0`, `FUN_005e68a0 @ 0x005E68A0`, `VocClass__PlayAtPos @ 0x00750920`, assembly around `0x005E69C2..0x005E69EC`.
- Existing docs reconciled: `SKIRMISH_BUTTON_CLICK_SOUND_PARITY_GHIDRA_REPORT.md`, `SKIRMISH_OWNER_DRAW_BUTTON_PRESS_RELEASE_TRACE.md`, `SKIRMISH_CHOOSE_MAP_MODAL_VISUAL_CONTROL_LAYOUT_GHIDRA_REPORT.md`, `SKIRMISH_CHOOSE_MAP_MODAL_0X6B_VISUAL_INTEGRATION_GHIDRA_REPORT.md`, `SKIRMISH_OWNERDRAW_BUTTON_PIXEL_LAYOUT_GHIDRA_REPORT.md`, `SKIRMISH_CHROME_CONTROL_ART_SUBSTITUTIONS_GHIDRA_REPORT.md`.
- INI checked: `ini/rulesmd.ini`, `ini/rules.ini`.
- Rust scanned read-only: `src/app.rs`, `src/rules/ruleset.rs`, `src/ui/skirmish_shell/state.rs`, `src/app_skirmish_shell_render.rs`.
