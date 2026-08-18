# Skirmish Choose Map 0x6B Keyboard / Default / Dismissal Current Rust Recheck - Ghidra Research Report

**Address(es):** `0x005E68A0`, callback bytes `0x005E6920..0x005E7041`, `0x005E7160`, `0x00612B70`, `0x00775700`, `0x007759E0`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** standard offline YR Choose Map dialog `0x6B` keyboard/default/double-click dismissal behavior compared against current Rust input code.  
**Non-Scope:** modal visual geometry, status/help hover mapping, Create Random Map generator internals, preview image lifecycle, and broader parent `0x102` command handling except for parent/global Escape suppression while the modal owns input.  
**Confidence:** High for binary callback/pump/command facts and current Rust Escape-suppression status; Medium for exact focused child-control keyboard behavior delegated to the original Windows child proc.  
**Active in YR:** Yes for the standard offline Skirmish Choose Map path; Conditional for branches requiring player button/list interaction.

## 0. Working Gate

- Target question: Does active standard YR Choose Map `0x6B` accept/dismiss through Enter, Escape, default buttons, list double-clicks, or only through real button command IDs, and does current Rust match?
- Non-goals: Do not re-open button geometry, modal asset composition, hover status mapping, random-map generation after command `0x583`, or scenario preview loading.
- Evidence needed to mark COMPLETE: live dialog creation proof, modal pump keyboard translation proof, callback command/message dispatch proof, resource/default-button facts from prior extraction, list/button double-click boundaries, and current Rust keyboard/mouse path comparison.
- Stop conditions: stop after every material keyboard/default/double-click/result behavior is resolved or marked delegated to standard Windows child control behavior.

## 1. Overview

The native `0x6B` modal does not provide a custom global Enter, Escape, or default-button path. The active callback recognizes real `WM_COMMAND` control IDs only: Use Map `0x6C5`, Cancel `0x5C0`, conditional Create Random Map `0x583`, and mode-list selection-change `0x6EB` with notification high word `1`. Current Rust now matches the important parent-input boundary: while `choose_map_modal` is open, the global native-shell Escape close path is consumed instead of closing the parent shell.

## 2. Verified Binary Findings

| Finding | Evidence | Active in YR |
|---|---|---|
| `0x6B` is created by the standard Choose Map opener and uses callback entry `0x005E6920`. | `0x005E68A0` decompile shows dialog id `0x6B`, callback `0x005E6920`, setup `FUN_00622820`, init `0x4A9`, show, and `FUN_007759E0` pump. Assembly `0x005E68B7..0x005E690F` confirms the same call path. | Yes |
| The callback has no custom key-message handling. | Callback bytes compare message to `0x111`, branch to `WM_COMMAND`, otherwise use a jump table for `0x0F`, `0x14`, `0x2B`, `0x82`; `WM_KEYDOWN 0x100`, `WM_KEYUP 0x101`, and `WM_CHAR 0x102` fall through to `0x005E7038`. Evidence: `0x005E6920..0x005E694C`, selector table noted in prior report at `0x005E7058`. | Yes |
| The modal pump does not run a dialog-key/default-button translator. | `0x007759E0` decompile loops messages with `TranslateMessage` and `DispatchMessageA`; no `IsDialogMessage` or accelerator translation is present. | Yes |
| The modal resource has no `IDOK`, `IDCANCEL`, or default pushbutton. | Prior resource extraction in `SKIRMISH_CHOOSE_MAP_MODAL_VISUAL_CONTROL_LAYOUT_GHIDRA_REPORT.md`: buttons are owner-draw `0x6C5`, `0x583`, `0x5C0`. | Yes |
| Button command actions are keyed by real control IDs, not aliases. | `0x005E69B7..0x005E69EC` masks `wParam` low word and checks `0x6C5`, `0x583`, then `0x5C0`; no low-word `1` or `2` alias branch. | Yes |
| Cancel closes with modal result `2`; Use Map calls accept helper. | Cancel path loads `EDX=2` then calls `0x007757E0` at `0x005E69E3..0x005E69EC`; Use Map path calls `0x005E7160` at `0x005E6B63..0x005E6B67`. | Yes |
| Use Map accepts only if the map list has a selected row. | `0x005E7160` sends `LB_GETCURSEL 0x188` to `0x553`, returns `0` on `0xffffffff`, then uses `LB_GETITEMDATA 0x199` before committing globals and closing. | Yes |
| Map-list double-click does not accept. | A listbox double-click would arrive as `WM_COMMAND` low word `0x553`, high word `2`; the callback has no `0x553` branch in the complete command dispatch. | Yes, as negative fact |
| Mode-list double-click is not a separate action. | `0x6EB` branch requires high word `1` at `0x005E6B90..0x005E6B96`; high word `2` falls through. | Conditional on player double-clicking mode list |
| Button double-click has no special modal result branch. | `OwnerDraw_Button_00612B70` treats `WM_LBUTTONDOWN 0x201` and `WM_LBUTTONDBLCLK 0x203` together for sound/down-state, then delegates; modal callback still acts only on the resulting real command ID. | Conditional on player double-clicking a button |

## 3. Current Rust Implementation Status

| Rust surface | Current status | Delta versus native |
|---|---|---|
| `src/app.rs:1419..1425` keyboard branch | If native Skirmish shell is active and Escape is pressed while `choose_map_modal` or `validation_modal` exists, Rust requests redraw and returns. | Now aligned for parent/global Escape suppression; earlier Escape-leak handoff is stale for current Rust. |
| `src/app.rs:1430..1435` shell key input routing | Shell edit-key handling is skipped for Escape and only calls `handle_skirmish_shell_key_input` for non-Escape pressed keys. | No modal Enter-to-Use-Map path observed; aligned with no custom global Enter/default accept. |
| `src/app.rs:1013..1050` `handle_skirmish_shell_key_input` | Only player-name edit consumes text/navigation while focused. | No Choose Map keyboard accept/cancel implementation observed; aligned for this scoped behavior. |
| `src/app.rs:781..841` modal mouse down | Real modal buttons become `pressed_button`; list row clicks select rows only; no double-click-specific code path exists. | Aligned for list double-click not accepting. |
| `src/app.rs:843..880` modal mouse up | Release-over-same real button ID fires Use Map, Cancel, or recognized `0x583`; Use Map commits `accept_selection`, Cancel closes, `0x583` logs only. | Command boundaries aligned for `0x6C5`/`0x5C0`/`0x583`; downstream random-map implementation remains out of scope and still incomplete. |
| `src/ui/skirmish_shell/state.rs:262..270` accept/cancel selection helpers | Accept returns highlighted map/mode; cancel helper preserves saved selection but app Cancel simply closes without committing changes. | Aligned for modal-local selection changes not affecting parent until Use Map. |

## 4. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `0x005E68A0` Choose Map opener | verified | decompile and assembly context | none |
| callback entry bytes `0x005E6920..0x005E7041` | verified | assembly context and prior selector-table decode | no function boundary was created because this swarm is read-only |
| Native key-message handling | verified | callback jump table excludes key messages | exact Windows focused-child key behavior remains delegated |
| Modal pump `0x007759E0` | verified | decompile has `TranslateMessage`/`DispatchMessageA` only | none |
| Resource default/cancel controls | verified | prior resource extraction | none |
| Use Map / Cancel / Random command IDs | verified | `0x005E69B7..0x005E69EC`, `0x005E6B63`, `0x005E7160` | random-map downstream out of scope |
| Map-list `0x553` double-click | verified | no `0x553` command branch | none |
| Mode-list `0x6EB` double-click | verified | high-word compare to `1` | none |
| Current Rust parent Escape suppression | verified | `src/app.rs:1419..1425` | add focused acceptance test if desired |
| Current Rust Enter/default behavior | verified | `src/app.rs:1013..1050`, `1430..1435` | add focused acceptance test if desired |

## 5. Open Questions - Final State

- `[RESOLVED] OQ-01 - Is dialog `0x6B` live in standard YR Choose Map? -> Yes, `0x005E68A0` creates resource `0x6B` with callback `0x005E6920` and pumps it.` (evidence: `0x005E68A0`, `0x005E68B7..0x005E690F`)
- `[RESOLVED] OQ-02 - Does callback `0x005E6920` handle Enter/Escape/key messages? -> No custom key-message handling; key messages fall through.` (evidence: `0x005E6920..0x005E694C`, `0x005E7038..0x005E7041`)
- `[RESOLVED] OQ-03 - Does the modal pump translate dialog keys into default/cancel commands? -> No; only `TranslateMessage` and `DispatchMessageA` are visible.` (evidence: `0x007759E0`)
- `[RESOLVED] OQ-04 - Does resource `0x6B` define `IDOK`, `IDCANCEL`, or a default pushbutton? -> No; real buttons are `0x6C5`, `0x583`, and `0x5C0`.` (evidence: `SKIRMISH_CHOOSE_MAP_MODAL_VISUAL_CONTROL_LAYOUT_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-05 - Which button command IDs are accepted? -> `0x6C5`, `0x5C0`, conditional `0x583`; no aliases `1`/`2`.` (evidence: `0x005E69B7..0x005E69EC`)
- `[RESOLVED] OQ-06 - Does Use Map require a selected map row? -> Yes; `0x005E7160` returns before close when `LB_GETCURSEL` is `0xffffffff`.` (evidence: `0x005E7160`)
- `[RESOLVED] OQ-07 - Does map-list double-click accept? -> No; no `0x553` command branch.` (evidence: `0x005E69B7..0x005E6B96`)
- `[RESOLVED] OQ-08 - Does mode-list double-click act specially? -> No separate action; `0x6EB` requires notification high word `1`.` (evidence: `0x005E6B90..0x005E6B96`)
- `[RESOLVED] OQ-09 - Does current Rust still let global shell Escape leak through while Choose Map is open? -> No; the current keyboard branch consumes Escape when `choose_map_modal` is present.` (evidence: `src/app.rs:1419..1425`)
- `[RESOLVED] OQ-10 - Does current Rust implement global Enter/default accept for Choose Map? -> No modal Enter/default path was found; shell key input is player-name-edit scoped.` (evidence: `src/app.rs:1013..1050`, `1430..1435`)
- `[DEFERRED] OQ-11 - Exact focused child-control Space/Enter behavior for owner-draw buttons.` (category: `needs-runtime-debugger`; reason: native path delegates to the original Windows child proc after Westwood owner-draw handling; next-step-if-pursued: runtime probe focused Use Map/Cancel button with Space/Enter)
- `[DEFERRED] OQ-12 - Winit-level double-click event sequence in current Rust.` (category: `out-of-scope`; reason: current app code has no double-click-specific modal path; exact OS event sequence is not needed for this command-boundary recheck; next-step-if-pursued: browser/manual event trace if double-click support is later added)

## 6. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Escape has no native modal cancel/default route and must not fall through to parent shell close while `0x6B` owns input. | callback excludes key handling; `0x007759E0`; resource has no `IDCANCEL`; current Rust `src/app.rs:1419..1425` | none observed in current Rust; prior leak is fixed | `src/app.rs` keyboard branch | Preserve modal Escape consumption while `choose_map_modal` is open. | `choose_map_modal_escape_is_consumed_without_closing_shell`: open Choose Map, press Escape, modal and parent shell remain. | Do not route Escape to parent Back or to modal Cancel without new runtime evidence. |
| Enter/default accept is not a native global modal action. | callback excludes key handling; pump lacks `IsDialogMessage`; no default pushbutton | none observed | `src/app.rs::handle_skirmish_shell_key_input` and keyboard event branch | Keep Enter from committing Use Map unless a focused child-control runtime probe proves a narrower native path. | `choose_map_modal_enter_does_not_accept_selection`: select a different map row, press Enter, then Cancel; parent map remains unchanged. | Do not add convenience `Enter = Use Map`. |
| List double-clicks do not accept or dismiss the modal. | no `0x553` command branch; `0x6EB` requires high word `1` | none observed | modal listbox input in `src/app.rs` | Keep row mouse interaction as selection/rebuild only. | `choose_map_modal_map_double_click_only_selects`: double-click map row, then Cancel; parent map unchanged. | Do not make map-row double-click fire Use Map. |
| Button actions are real command IDs only. | `0x005E69B7..0x005E69EC`, `0x005E6B63`, `0x005E7160` | aligned for mouse release-over-same IDs; `0x583` downstream remains incomplete outside this slice | `src/app.rs::handle_choose_map_modal_mouse_up` | Continue firing only `UseMap0x6c5`, `Cancel0x5c0`, `CreateRandomMap0x583` from modal buttons. | `choose_map_modal_buttons_fire_only_real_ids`: Use Map commits, Cancel closes without commit, random command does not masquerade as accept/cancel. | Do not introduce `IDOK`/`IDCANCEL` aliases. |

## 7. Negative Facts / Do Not Do

- Do not implement global `Enter = Use Map`. Active in YR: No.
- Do not implement global `Escape = Cancel`. Active in YR: No.
- Do not let parent/global Escape close the native Skirmish shell while Choose Map is open. Active in YR equivalent: modal owns input; current Rust now suppresses this.
- Do not treat map-list `0x553` double-click as accept. Active in YR: No.
- Do not treat mode-list `0x6EB` high word `2` as selection-change or accept. Active in YR: No; only high word `1` is accepted.
- Do not add `IDOK`/`IDCANCEL` aliases to the resource/control model. Active in YR: No.

## 8. Remaining Uncertainty

- Exact focused child-control keyboard behavior for owner-draw buttons remains runtime-only because Westwood delegates unhandled keys to the original child proc. This does not justify a global Enter/Escape shortcut.
- The exact Winit double-click message cadence was not measured. Current Rust has no double-click-specific modal branch, which is sufficient for this scoped recheck.

## 9. Proposed Rust Test Names

- `choose_map_modal_escape_is_consumed_without_closing_shell`
- `choose_map_modal_enter_does_not_accept_selection`
- `choose_map_modal_map_double_click_only_selects`
- `choose_map_modal_mode_double_click_does_not_fire_command`
- `choose_map_modal_buttons_fire_only_release_over_same_real_control_id`

## 10. Stale Docs / Follow-Up Docs

- Replace stale wording from `SKIRMISH_CHOOSE_MAP_0X6B_KEYBOARD_DEFAULT_DISMISSAL_GHIDRA_REPORT.md` handoff row: "current global Escape can close native shell while modal is open" with "current Rust now consumes global Escape while `choose_map_modal` or `validation_modal` owns input (`src/app.rs:1419..1425`); preserve this guard with an acceptance test."
- Keep the prior negative facts for Enter/default/list double-click behavior; they remain valid.

## Sources

- Ghidra decompile: `0x005E68A0`, `0x005E7160`, `0x00612B70`, `0x00775700`, `0x007759E0`.
- Ghidra assembly context: `0x005E6920`, `0x005E6930`, `0x005E69B7`, `0x005E69E3`, `0x005E6B63`, `0x005E6B90`, `0x005E7038`.
- Prior report: `docs/research/skirmish-ui/SKIRMISH_CHOOSE_MAP_0X6B_KEYBOARD_DEFAULT_DISMISSAL_GHIDRA_REPORT.md`.
- Prior resource extraction: `docs/research/skirmish-ui/SKIRMISH_CHOOSE_MAP_MODAL_VISUAL_CONTROL_LAYOUT_GHIDRA_REPORT.md`.
- Rust scanned read-only: `src/app.rs`, `src/ui/skirmish_shell/state.rs`.
