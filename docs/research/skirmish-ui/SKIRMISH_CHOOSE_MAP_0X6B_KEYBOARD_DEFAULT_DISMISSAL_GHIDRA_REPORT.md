# Skirmish Choose Map 0x6B Keyboard / Default / Dismissal - Ghidra Research Report

**Address(es):** `0x005E68A0`, callback bytes `0x005E6920..0x005E7041`, `0x005E7160`, `0x00612B70`, `0x00775700`, `0x007759E0`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** standard offline YR Choose Map dialog `0x6B` keyboard/default/dismissal behavior: Enter, Escape, default/cancel button identity, Use Map/Cancel result boundaries, list/button double-click behavior, and the command/result boundary for Create Random Map `0x583`.  
**Non-Scope:** listbox visuals, full random-map generation, random-map setup dialog layout, status/help hover text, preview decode/lifecycle beyond whether `0x583` has a distinct acceptance path, and broader parent `0x102` behavior except for parent Escape suppression while the modal owns input.  
**Confidence:** High for binary callback/pump/command facts and current Rust comparison; Medium for exact focused child-control Space/Enter behavior delegated to the original Windows child proc.  
**Active in YR:** Yes for the standard offline Skirmish Choose Map path; Conditional for branches requiring player button/list interaction.

## 0. Working Notes Gate

- Target question: Does active standard YR Choose Map `0x6B` accept/dismiss through Enter, Escape, default buttons, list double-clicks, or only through real button command IDs, and does current Rust match?
- Non-goals: Do not re-open button/listbox visuals, modal asset composition, hover status mapping, random-map generation after command `0x583`, or scenario preview loading.
- Evidence needed to mark COMPLETE: live dialog creation proof, modal pump keyboard translation proof, callback command/message dispatch proof, resource/default-button facts, list/button double-click boundaries, random-map setup result boundary, and current Rust keyboard/mouse comparison.
- Stop conditions: stop after every material keyboard/default/double-click/result behavior is resolved from binary/resource/Rust evidence or marked delegated to standard Windows child-control behavior.

Prior state row: **Partial/high-confidence reports exist; proceed to gaps + verification only.** This report refreshes the earlier keyboard/default report against current Rust after Choose Map modal and MPModes work; it replaces stale handoff wording that said parent Escape could still close the shell while the chooser is open.

## 1. Overview

The native `0x6B` modal does not implement a custom global Enter, Escape, IDOK, IDCANCEL, or default-pushbutton path. The live callback recognizes real `WM_COMMAND` control IDs only: Use Map `0x6C5`, Cancel `0x5C0`, conditional Create Random Map `0x583`, and mode-list selection-change `0x6EB` only when the notification high word is `1`.

Current Rust now matches the important parent-input boundary: while `choose_map_modal` is open, Escape is consumed and does not close the parent native shell. Rust also has no global Enter-to-Use-Map path and no list double-click accept path, which matches the verified native command boundary.

## 2. Verified Binary Findings

| Finding | Evidence | Active in YR |
|---|---|---|
| `0x6B` is the live standard Choose Map dialog. `0x005E68A0` creates dialog resource `0x6B` through `0x00775700`, passes callback `0x005E6920`, subclasses/initializes children, sends custom init `0x4A9`, shows the dialog, and pumps `0x007759E0`. | Ghidra decompile `0x005E68A0`; disassembly range `0x005E6920..0x005E7041` exists but has no function boundary, so it was inspected read-only without creating one. | Yes |
| The callback has no custom key-message handling. It special-cases `WM_COMMAND (0x111)` and has a small lower-message jump table for paint/erase/draw/cleanup messages; `WM_KEYDOWN (0x100)`, `WM_KEYUP (0x101)`, and `WM_CHAR (0x102)` fall through. | Prior selector-table decode at `0x005E7044`/`0x005E7058`; fresh disassembly coverage `0x005E6920..0x005E7041`. | Yes |
| The modal pump does not translate dialog keys into default/cancel actions. It loops messages through `TranslateMessage` and `DispatchMessageA`; no `IsDialogMessage`, `TranslateAccelerator`, or custom key prefilter is present. | Ghidra decompile `0x007759E0`. | Yes |
| Resource `0x6B` has no `IDOK`, `IDCANCEL`, or default pushbutton. Its right-column buttons are owner-draw `BUTTON` controls with ids `0x6C5`, `0x583`, and `0x5C0`. | `SKIRMISH_CHOOSE_MAP_MODAL_VISUAL_CONTROL_LAYOUT_GHIDRA_REPORT.md`; button style `0x5000000B`. | Yes |
| Button command actions are keyed by real control IDs, not aliases. The command dispatch masks `wParam` low word and checks `0x6C5`, `0x583`, then `0x5C0`; there are no low-word `1` or `2` branches. | Disassembly range `0x005E69B7..0x005E6B96`; prior report branch decode. | Yes |
| Cancel closes with modal result `2`. | Cancel path loads result `2` then calls modal close helper `0x007757E0` in `0x005E69E3..0x005E69EC`; parent `0x006ACEE0` compares modal result to literal `2` in the accept/cancel side-effects report. | Yes |
| Use Map calls `0x005E7160`; that helper returns `0` without closing if the map list `0x553` has no selected row, if item data cannot be matched to a scenario record, or if the resolved index is invalid. On success it commits selected globals and closes with result `1`. | Ghidra decompile `0x005E7160`; parent return contract in `SKIRMISH_CHOOSE_MAP_ACCEPT_CANCEL_SIDE_EFFECTS_GHIDRA_REPORT.md`. | Yes |
| Map-list double-click does not accept. A listbox double-click would be `WM_COMMAND` low word `0x553`, high word `2`; the callback has no `0x553` command branch. | Complete command dispatch coverage `0x005E69B7..0x005E6B96`. | Yes, as a negative fact |
| Mode-list double-click is not a separate action. The `0x6EB` branch accepts only notification high word `1`; high word `2` falls through. | Branch at `0x005E6B90..0x005E6B96` from prior decode plus fresh command range coverage. | Conditional on player double-clicking the mode list |
| Button double-click has no special modal result branch. The shared owner-draw button proc treats `WM_LBUTTONDOWN (0x201)` and `WM_LBUTTONDBLCLK (0x203)` together for sound/down-state behavior, then delegates to the original child proc; the modal callback still acts only on resulting real command IDs. | Ghidra decompile `0x00612B70`; command dispatch `0x005E69B7..0x005E6B96`. | Conditional on player double-clicking a button |
| Create Random Map `0x583` is a real command branch, but its side effects are gated by a distinct random-map setup acceptance path, not by Enter/default or mere button press. | Command dispatch range `0x005E69B7..0x005E6B96`; `SKIRMISH_CREATE_RANDOM_MAP_0X583_IMPLEMENTATION_CONTRACT_GHIDRA_REPORT.md` verifies `FUN_005E8590` returns `-1` unless the random-map setup returns exactly `1`. | Conditional on player clicking `0x583` and accepting setup |

## 3. INI Keys

No INI key is directly read by this keyboard/default/dismissal slice. `0x583` admission by selected mode's random-map flag is covered by the random-map implementation-contract report and MPModes reports; generation internals are out of scope here.

## 4. Integration Points

| Integration point | Behavior | Evidence | Active in YR |
|---|---|---|---|
| Choose Map opener | Creates `0x6B`, initializes/subclasses it, shows it, then blocks in the modal pump. | `0x005E68A0`, `0x00775700`, `0x007759E0` | Yes |
| Dialog callback | Handles command ids and paint/draw/cleanup messages; unhandled keyboard messages fall through. | callback bytes `0x005E6920..0x005E7041` | Yes |
| Button owner proc | Handles down/double-click visual/sound state before delegating to original button proc. | `0x00612B70` | Yes |
| Parent return path | Treats modal result `2` as cancel/restore; accepted Use Map result commits and refreshes parent state. | `SKIRMISH_CHOOSE_MAP_ACCEPT_CANCEL_SIDE_EFFECTS_GHIDRA_REPORT.md` | Yes |

## 5. Current Rust Implementation Status

| Rust surface | Current status | Delta versus native |
|---|---|---|
| `src/app.rs:1481..1490` keyboard branch | If native Skirmish shell is active and Escape is pressed while `choose_map_modal` or `validation_modal` exists, Rust requests redraw and returns. | Aligned for parent/global Escape suppression; the old "Escape leaks to parent shell" handoff is stale. |
| `src/app.rs:1492..1498` shell key routing | Shell key input is called only for non-Escape pressed keys. | Aligned for no modal Escape cancel path. |
| `src/app.rs:1070..1114` `handle_skirmish_shell_key_input` | Only focused player-name edit consumes text/navigation; no Choose Map Enter/default handling was found. | Aligned for no global Enter-to-Use-Map path. |
| `src/app.rs:789..849` modal mouse down | Real modal buttons set `pressed_button`; list row clicks select rows; no double-click-specific path exists. | Aligned for list double-click not accepting. |
| `src/app.rs:851..887` modal mouse up | Release over the same real modal button fires Use Map, Cancel, or recognized `0x583`; Use Map commits `accept_selection`, Cancel closes, `0x583` is still not a full random setup implementation. | Command IDs aligned; random setup remains a separate gap outside this keyboard/dismissal slice. |
| `src/ui/skirmish_shell/state.rs:273..282` accept/cancel helpers | Accept returns highlighted mode/map; cancel preserves saved selection. App Cancel closes without committing highlights. | Aligned for transaction boundary. |

## 6. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `0x005E68A0` Choose Map opener | verified | Ghidra decompile | none |
| callback entry bytes `0x005E6920..0x005E7041` | verified | disassembly range plus prior selector decode | no function boundary was created because this swarm is read-only |
| key-message handling | verified | callback jump table excludes key messages | focused child Space/Enter behavior remains delegated |
| modal pump `0x007759E0` | verified | Ghidra decompile | none |
| resource default/cancel identity | verified | visual-control layout report | none |
| Use Map / Cancel / Random command IDs | verified | `0x005E69B7..0x005E6B96`, `0x005E7160` | random-map downstream generation out of scope |
| Use Map no-selection boundary | verified | `0x005E7160` returns before close on `LB_GETCURSEL == 0xffffffff` | none |
| map-list `0x553` double-click | verified | no `0x553` command branch | none |
| mode-list `0x6EB` double-click | verified | branch requires high word `1` | none |
| button double-click | verified | `0x00612B70`; modal command dispatch has no double-click result branch | exact child proc keyboard activation remains delegated |
| current Rust parent Escape suppression | verified | `src/app.rs:1481..1490` | add regression test |
| current Rust Enter/default behavior | verified | `src/app.rs:1070..1114`, `1492..1498` | add regression test |

## 7. Open Questions - Final State

- `[RESOLVED] OQ-01 - Is dialog `0x6B` live in standard YR Choose Map? -> Yes, `0x005E68A0` creates resource `0x6B` with callback `0x005E6920` and pumps it.` (evidence: `0x005E68A0`, `0x00775700`, `0x007759E0`)
- `[RESOLVED] OQ-02 - Does callback `0x005E6920` handle Enter/Escape/key messages? -> No custom key-message handling; key messages fall through.` (evidence: `0x005E6920..0x005E7041`)
- `[RESOLVED] OQ-03 - Does the modal pump translate dialog keys into default/cancel commands? -> No; only `TranslateMessage` and `DispatchMessageA` are visible.` (evidence: `0x007759E0`)
- `[RESOLVED] OQ-04 - Does resource `0x6B` define `IDOK`, `IDCANCEL`, or a default pushbutton? -> No; real buttons are `0x6C5`, `0x583`, and `0x5C0`.` (evidence: `SKIRMISH_CHOOSE_MAP_MODAL_VISUAL_CONTROL_LAYOUT_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-05 - Which button command IDs are accepted? -> `0x6C5`, `0x5C0`, conditional `0x583`; no aliases `1`/`2`.` (evidence: `0x005E69B7..0x005E6B96`)
- `[RESOLVED] OQ-06 - Does Use Map require a selected map row? -> Yes; `0x005E7160` returns before close when `LB_GETCURSEL` is `0xffffffff`.` (evidence: `0x005E7160`)
- `[RESOLVED] OQ-07 - Does map-list double-click accept? -> No; no `0x553` command branch.` (evidence: `0x005E69B7..0x005E6B96`)
- `[RESOLVED] OQ-08 - Does mode-list double-click act specially? -> No separate action; `0x6EB` requires notification high word `1`.` (evidence: `0x005E6B90..0x005E6B96`)
- `[RESOLVED] OQ-09 - Does current Rust still let global shell Escape leak through while Choose Map is open? -> No; the current keyboard branch consumes Escape when `choose_map_modal` is present.` (evidence: `src/app.rs:1481..1490`)
- `[RESOLVED] OQ-10 - Does current Rust implement global Enter/default accept for Choose Map? -> No modal Enter/default path was found; shell key input is player-name-edit scoped.` (evidence: `src/app.rs:1070..1114`, `1492..1498`)
- `[RESOLVED] OQ-11 - Does random-map command have a distinct accept path? -> Yes; `0x583` enters setup and only accepted setup result produces side effects; no keyboard/default path substitutes for it.` (evidence: `0x005E69B7..0x005E6B96`; `SKIRMISH_CREATE_RANDOM_MAP_0X583_IMPLEMENTATION_CONTRACT_GHIDRA_REPORT.md`)
- `[DEFERRED] OQ-12 - Exact focused child-control Space/Enter behavior for owner-draw buttons.` (category: `needs-runtime-debugger`; reason: native path delegates unhandled key behavior to the original Windows child proc; next-step-if-pursued: runtime probe focused Use Map/Cancel/Create Random button with Space/Enter)
- `[DEFERRED] OQ-13 - Exact Winit double-click event sequence in current Rust.` (category: `out-of-scope`; reason: current app code has no double-click-specific modal path; exact OS event cadence is not needed for this command-boundary recheck; next-step-if-pursued: manual event trace if double-click support is later added)

## 8. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Escape has no native modal cancel/default route and must not fall through to parent shell close while `0x6B` owns input. | callback excludes key handling; `0x007759E0`; resource has no `IDCANCEL`; current Rust `src/app.rs:1481..1490` | none observed in current Rust | `src/app.rs` keyboard branch | Preserve modal Escape consumption while `choose_map_modal` is open. | Open Choose Map, press Escape: modal and parent shell remain. Proposed test: `choose_map_modal_escape_is_consumed_without_closing_shell`. | Do not route Escape to parent Back or to modal Cancel without new runtime evidence. |
| Enter/default accept is not a native global modal action. | callback excludes key handling; pump lacks `IsDialogMessage`; resource has no default pushbutton | none observed | `src/app.rs::handle_skirmish_shell_key_input` and keyboard event branch | Keep Enter from committing Use Map unless a focused child-control runtime probe proves a narrower native path. | Select a different map row, press Enter, then Cancel; parent map remains unchanged. Proposed test: `choose_map_modal_enter_does_not_accept_selection`. | Do not add convenience `Enter = Use Map`. |
| Use Map closes only after a valid selected map item resolves to a scenario record; no selected row is a no-close/no-commit path. | `0x005E7160` | current Rust `accept_selection` returns `None` on no selected row and mouse-up then does not close; aligned | `src/app.rs::handle_choose_map_modal_mouse_up`, `ChooseMapModalState::accept_selection` | Preserve no-close behavior when Use Map has no valid selected record. | Open chooser with empty filtered list, release Use Map: modal remains open and committed map unchanged. Proposed test: `choose_map_modal_use_map_without_selection_does_not_close`. | Do not treat Use Map as Cancel or accept a synthetic null map. |
| List double-clicks do not accept or dismiss the modal. | no `0x553` command branch; `0x6EB` requires high word `1` | none observed | modal listbox input in `src/app.rs` | Keep row mouse interaction as selection/rebuild only. | Double-click a map row, then Cancel; parent map unchanged. Proposed test: `choose_map_modal_map_double_click_only_selects`. | Do not make map-row double-click fire Use Map. |
| `0x583` is a real button command but has a distinct setup acceptance gate. | command dispatch `0x005E69B7..0x005E6B96`; random-map contract report | random setup/generation still incomplete, but this slice only verifies keyboard/default boundaries | `src/app.rs::handle_choose_map_modal_mouse_up`; future random-map setup surface | Keep `0x583` separate from Use Map/Cancel and gate side effects on accepted setup result, not on default Enter or button-down alone. | Click Create Random Map, cancel setup: previous committed map/mode remain; accepted setup commits `RandMap.Sed` through random-map contract. Proposed test: `choose_map_create_random_map_cancel_preserves_previous_selection`. | Do not make `0x583` a shortcut for Use Map, and do not create/update `RandMap.Sed` on mere click. |

## 9. Negative Facts / Do Not Do

- Do not implement global `Enter = Use Map`. Active in YR: No; callback/pump/resource provide no such path (`0x005E6920..0x005E7041`, `0x007759E0`, resource report).
- Do not implement global `Escape = Cancel`. Active in YR: No; no key path and no `IDCANCEL` resource control.
- Do not let parent/global Escape close the native Skirmish shell while Choose Map is open. Active in YR equivalent: modal pump owns input until close; current Rust already suppresses this (`src/app.rs:1481..1490`).
- Do not treat map-list `0x553` double-click as accept. Active in YR: No; no `0x553` command branch.
- Do not treat mode-list `0x6EB` high word `2` as selection-change or accept. Active in YR: No; only high word `1` is accepted.
- Do not add `IDOK`/`IDCANCEL` aliases to the resource/control model. Active in YR: No; real controls are `0x6C5`, `0x583`, and `0x5C0`.
- Do not use Create Random Map `0x583` as a keyboard/default acceptance shortcut. Active in YR: No; it has a separate setup return-value gate.

## 10. Remaining Uncertainty

- Exact focused child-control keyboard behavior for owner-draw buttons remains runtime-only because Westwood delegates unhandled keys to the original child proc. This does not justify a global Enter/Escape shortcut.
- Exact Winit double-click message cadence was not measured. Current Rust has no double-click-specific modal branch, which is sufficient for this scoped command-boundary recheck.

## 11. Visual/UI Composition Ledger

This slice has no visual-composition claim beyond resource/control identity. Full composition remains owned by the visual-control/layout and modal-composition reports.

| Order | Function / address | Condition / flag proof | Asset / frame | Rect / anchor | Palette / convert | Active for target? | Role |
|---|---|---|---|---|---|---|---|
| 1 | resource `0x6B` controls | standard Choose Map opener `0x005E68A0` | none in this slice | buttons `0x6C5`, `0x583`, `0x5C0` per resource report | not claimed | yes | command identity only |

## 12. Stale Docs / Follow-Up Docs

- `docs/research/skirmish-ui/SKIRMISH_CHOOSE_MAP_0X6B_KEYBOARD_DEFAULT_DISMISSAL_CURRENT_RUST_RECHECK_GHIDRA_REPORT.md`: keep as an earlier recheck, but canonical wording is now in this report.
- Replacement wording for stale handoff in the previous version of this file: replace "current global Escape can close native shell while modal is open" with "current Rust consumes global Escape while `choose_map_modal` or `validation_modal` owns input (`src/app.rs:1481..1490`); preserve this guard with a regression test."
- Older Choose Map no-op/no-render traces should not be used for keyboard/dismissal decisions. Replacement wording: "Current Rust has a modal state and primitive render/input path; remaining keyboard/dismissal parity is specifically Enter/Escape/default/list-double-click command behavior, not absence of a modal."

## Sources

- Fresh Ghidra read-only decompile: `0x005E68A0`, `0x005E7160`, `0x00612B70`, `0x00775700`, `0x007759E0`.
- Fresh read-only Ghidra disassembly coverage: callback bytes `0x005E6920..0x005E7041`, command range `0x005E69B7..0x005E6B96`.
- Prior resource extraction: `docs/research/skirmish-ui/SKIRMISH_CHOOSE_MAP_MODAL_VISUAL_CONTROL_LAYOUT_GHIDRA_REPORT.md`.
- Prior side-effect/return reports: `docs/research/skirmish-ui/SKIRMISH_CHOOSE_MAP_ACCEPT_CANCEL_SIDE_EFFECTS_GHIDRA_REPORT.md`, `docs/research/skirmish-ui/SKIRMISH_CREATE_RANDOM_MAP_0X583_IMPLEMENTATION_CONTRACT_GHIDRA_REPORT.md`.
- Current Rust scanned read-only: `src/app.rs`, `src/ui/skirmish_shell/state.rs`.
