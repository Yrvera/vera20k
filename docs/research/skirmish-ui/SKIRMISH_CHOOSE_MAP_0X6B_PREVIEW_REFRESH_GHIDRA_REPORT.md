# Skirmish Choose Map 0x6B Preview Refresh - Ghidra Research Report

**Address(es):** `0x005E6920` label/disassembly slice, `0x005E68A0`, `0x005E7160`, `0x005E70D0`, `0x005E74E0`, `0x00640710`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** dialog `0x6B` callback entry `0x005E6920`: `WM_PAINT`, `WM_COMMAND` branches for controls `0x553`, `0x6EB`, `0x583`, `0x5C0`, `0x6C5`, and preview-object calls needed to answer whether preview control `0x468` updates while browsing/highlighting rows before Use Map commits.  
**Non-Scope:** parent setup `0x102` return refresh except as contrast, full row paint visuals, full random-map generation internals, full `0x6B` geometry, and non-offline lobby variants.  
**Confidence:** High for no normal row-highlight preview refresh; Medium-High for the `0x005E6920` branch map because Ghidra has no function boundary there, but retail bytes were inspected read-only and helpers were decompiled.  
**Active in YR:** Yes. The parent `0x005E68A0` creates resource `0x6B` with callback label `0x005E6920`, stores the chooser HWND in `DAT_00AC0D40`, sends init message `0x4A9`, shows the chooser, and enters the modal pump.

## Working Notes Gate

- Target question: Does Choose Map dialog `0x6B` update preview static/control `0x468` while browsing/highlighting rows in listbox `0x553` before Use Map commits?
- Non-goals: Do not re-investigate parent `0x102` accepted/cancel preview refresh, modal geometry, row owner-draw pixels, combo/listbox visual paint, or random-map generation beyond preview-relevant calls in `0x005E6920`.
- Evidence needed to mark COMPLETE: map `0x005E6920` `WM_PAINT` and `WM_COMMAND` branches, prove whether `0x553` and `0x6EB` selection notifications call preview loaders/invalidation, and compare those calls to accepted `0x6C5` and parent refresh functions.
- Stop conditions: stop when every preview-affecting call in `0x005E6920` is classified or explicitly ruled out, and do not chase helper internals that only affect list ordering or row visuals.

## 1. Overview

Dialog `0x6B` has its own preview paint path, but it does not refresh the preview object when the player merely highlights rows in the map list. `WM_PAINT` draws whatever global preview wrapper `DAT_00AC1154` currently holds by calling `DrawStartPositions @ 0x00640710`; selection-change branches in `0x005E6920` rebuild or reselect listbox rows, but they do not call `0x005E7BF0`, `0x005E74E0`, `0x00641DB0`, or `InvalidateRect` for a newly highlighted row.

The normal player-visible result is that browsing/highlighting rows inside the chooser keeps showing the previously loaded preview until a commit or special Create Random Map flow changes state. The committed selection path starts at Use Map `0x6C5`: `0x005E7160` writes selected globals and closes the modal, then the already-documented parent `0x102` return path loads/invalidates the preview.

## 2. Key Controls / Globals

| Item | Meaning in this slice | Evidence | Active in YR |
|---|---|---|---|
| Dialog resource `0x6B` | Choose Map modal | `0x005E68A0` passes callback label `0x005E6920` to `0x00775700` | Yes |
| `0x468` | Preview anchor/static used by `DrawStartPositions` | `DrawStartPositions @ 0x00640710` calls `GetDlgItem(param_2, 0x468)` | Yes |
| `0x553` | Map listbox | `0x005E6920` list rebuild/select messages and `0x005E7160` accept reads `LB_GETCURSEL/LB_GETITEMDATA` from `0x553` | Yes |
| `0x6EB` | Mode/category listbox | `0x005E6920` handles `WM_COMMAND` low word `0x6EB`, notification high word `1` | Yes |
| `0x6C5` | Use Map button | `0x005E6920` command branch calls `0x005E7160` | Yes |
| `0x5C0` | Cancel button | `0x005E6920` command branch calls modal close helper with result `2` | Yes |
| `0x583` | Create Random Map button | `0x005E6920` command branch runs random-map creation/list update path | Conditional: active when the button is enabled by selected mode/category |
| `DAT_00AC1154` | Global preview wrapper pointer | `0x005E6920` `WM_PAINT`; loaders `0x005E74E0`, `0x00641DB0` | Yes |
| `DAT_00AC11C8` | Init guard suppressing category-selection handler | `0x005E6920` sets it during `0x497` init, tests it before `0x6EB` branch | Yes |
| `DAT_00AC11C9` / `DAT_00AC0EC8` | Saved text/key used to preserve/select a map-list row after category rebuild | `0x005E6920` branch `0x005E6CD4..0x005E6DB4` | Yes |

## 3. Core Logic

### 3.1 Modal creation proves activity

Active in YR: Yes. `0x005E68A0` constructs shell state, creates dialog resource `0x6B` through `0x00775700` with callback label `0x005E6920`, stores the HWND in `DAT_00AC0D40`, sends message `0x4A9`, shows the window, and enters modal pump `0x007759E0`.

Evidence: decompile of `0x005E68A0`; direct call from parent `0x006ACEE0` is already covered by `SKIRMISH_CHOOSE_MAP_PREVIEW_REFRESH_FUN_006ACEE0_GHIDRA_REPORT.md`.

### 3.2 Ghidra function-boundary note

Ghidra reports no function at address `0x005E6920`, but `0x005E68A0` references label `LAB_005E6920` as the dialog callback. The slice was therefore inspected via read-only bytes/disassembly at `0x005E6920..0x005E7041`, plus decompiled helpers reached from that disassembly. No Ghidra database mutation was performed.

Active in YR: Yes. Evidence: `0x005E68A0` decompile plus read-only disassembly of the callback label.

### 3.3 `WM_PAINT` draws current preview, not a newly highlighted row

Active in YR: Yes. The callback's `WM_PAINT` branch is reached from the jump table for message `0x0F`. It:

1. calls common shell paint helper `0x00621E90`;
2. tests `DAT_00AC1154`;
3. if non-null, calls `DrawStartPositions @ 0x00640710` with the chooser HWND;
4. calls `0x00607FD0`;
5. calls `ValidateRect(hwnd, NULL)`;
6. returns `0`.

Evidence: disassembly `0x005E696B..0x005E699F`; `DrawStartPositions @ 0x00640710` decompile validates the HWND, gets child `0x468`, computes the aspect-fitted preview, blits the inner preview surface, then draws start markers.

Important detail: `WM_PAINT` consumes `DAT_00AC1154`; it does not choose a scenario record from `0x553`, does not read the highlighted listbox row, and does not load a new preview.

### 3.4 Map-list `0x553` highlight has no normal `WM_COMMAND` handler

Active in YR: Yes as a negative fact for standard dialog processing. In the `WM_COMMAND` branch, the low-word command dispatch recognizes:

- `0x5C0` Cancel;
- `0x583` Create Random Map;
- `0x6C5` Use Map;
- `0x6EB` category list selection change when notification high word is `1`;
- default return for other command ids.

Control `0x553` is not handled in this `WM_COMMAND` dispatch. A plain listbox selection/highlight notification from `0x553` falls through the default branch and does not call any preview loader, preview object replacement, or invalidation routine.

Evidence: disassembly `0x005E69B7..0x005E69E3` and `0x005E6B78..0x005E6B96`; default return at `0x005E7038..0x005E7041`. This is stronger than absence-by-search because the active dispatch was read branch-by-branch.

### 3.5 Category-list `0x6EB` selection change rebuilds list `0x553`, but does not refresh preview

Active in YR: Conditional. This branch runs only when `DAT_00AC11C8 == 0` and the `0x6EB` notification high word is exactly `1`; it is suppressed during init.

The branch:

1. reads current `0x6EB` selection with `LB_GETCURSEL (0x188)`;
2. ignores unchanged selection by comparing against `DAT_008316FC`;
3. reads selected mode/category object with `LB_GETITEMDATA (0x199)`;
4. validates the selected object through vtable calls;
5. filters `DAT_00A8B8CC` scenario records with `0x005D63E0`;
6. appends matching records to a temporary vector with `0x005EEE40`;
7. repopulates/rebinds listbox `0x553` through the selected mode/category vtable `+0x4C`;
8. enables/disables `Create Random Map` control `0x583`;
9. preserves/reselects a row in `0x553` using `DAT_00AC0EC8` text comparison, `LB_SETCURSEL (0x186)`, and `LB_SETTOPINDEX (0x197)`;
10. clears `DAT_00AC11C9`.

The branch contains no calls to `0x005E7BF0`, `0x005E74E0`, `0x00641DB0`, `0x006406F0`, `0x006406E0`, `DrawStartPositions`, or `InvalidateRect`.

Evidence: disassembly `0x005E6B78..0x005E6DB4`; decompiles of `0x005D63E0` (record filter), `0x005EEE40` (append to vector), `0x005EED00` (temporary cleanup), and string helpers `0x007CA5D3`/`0x007CA489`.

### 3.6 Init `0x497` populates and selects rows; it does not create a browsing-preview contract

Active in YR: Yes. Message `0x497` sets `DAT_00AC11C8 = 1`, derives the current mode/category from `DAT_00A8B250`, populates `0x6EB`, populates `0x553` from records accepted by that category, enables/disables `0x583`, selects the current committed `DAT_00A8B254` record in `0x553`, writes `DAT_00AC10E0 = DAT_00A8B254`, sets `DAT_00AC0D30 = -1`, then clears `DAT_00AC11C8 = 0`.

Evidence: disassembly `0x005E6EA6..0x005E7031`. Preview-relevant negative evidence: this init branch does not call `0x005E74E0` or `InvalidateRect`; it relies on the current `DAT_00AC1154` for later paint.

### 3.7 Use Map commits; preview refresh happens after commit, not on browse

Active in YR: Yes. Command `0x6C5` calls `0x005E7160`. That helper reads the selected row from `0x553`, resolves item data back to a record in `DAT_00A8B8CC`, reads selected category from `0x6EB`, writes `DAT_00A8B23C`, `DAT_00A8B250`, and `DAT_00A8B254`, closes the modal through `0x007757E0`, and sends text update messages to `0x6EC` and `0x5A8`.

Evidence: disassembly `0x005E6B63..0x005E6B75`; decompile of `0x005E7160`. Preview-relevant negative evidence: `0x005E7160` does not call `0x005E74E0`; parent `0x006ACEE0` performs the accepted return preview refresh documented in `SKIRMISH_CHOOSE_MAP_PREVIEW_REFRESH_FUN_006ACEE0_GHIDRA_REPORT.md`.

### 3.8 Create Random Map is the exception, not row browsing

Active in YR: Conditional. Command `0x583` runs only when the button is enabled by the selected mode/category. Its branch hides the chooser, calls `0x005E8590`, rebuilds/reselects list rows, calls `0x005E70D0`, calls selected-record load `0x005E7BF0`, falls back to `0x005E74E0` when `DAT_00AC1154`'s inner pointer is null, restores `DAT_00A8B254` from `DAT_00AC10E0`, calls `0x005E7160`, and then shows the chooser again if the accept helper did not close/continue.

Evidence: disassembly `0x005E69FD..0x005E6B57`; decompiles of `0x005E70D0`, `0x005E7BF0`, `0x005E74E0`, and `0x005E7160`.

Implementation consequence: do not infer hover/list-highlight preview behavior from the Create Random Map button. It is a command path with map-generation side effects, not a passive selection-change path.

## 4. INI Keys

No INI keys are directly read by this `0x6B` preview-refresh slice. Filtering uses already-populated scenario records and mode/category objects. Preview loading after commit uses selected map data through existing preview-loader paths, covered by sibling reports.

## 5. Integration Points

| Integration point | Verified behavior | Evidence | Active in YR |
|---|---|---|---|
| Parent opens chooser | `0x005E68A0` creates dialog `0x6B` with callback label `0x005E6920` and pumps modal | `0x005E68A0` decompile | Yes |
| Chooser paint | Draws current `DAT_00AC1154` through `DrawStartPositions`, using child `0x468` as anchor | `0x005E696B..0x005E699F`, `0x00640710` decompile | Yes |
| Map-list browse | No `0x553` `WM_COMMAND` selection handler exists in `0x005E6920` | `0x005E69B7..0x005E69E3`, `0x005E6B78..0x005E6B96` | Yes as negative |
| Category change | Rebuilds `0x553` and preserves/reselects a row; does not reload preview | `0x005E6B78..0x005E6DB4` | Conditional |
| Use Map | Commits selected globals and closes modal; parent return path refreshes preview | `0x005E6B63..0x005E6B75`, `0x005E7160` | Yes |
| Create Random Map | Button command can load/fallback preview-related state, but is not browsing/highlight behavior | `0x005E69FD..0x005E6B57` | Conditional |

## 6. Current Rust Implementation Status

Rust has a chooser-state skeleton whose highlight can differ from the committed selection, which matches the binary's no-commit-on-browse direction. The app/render path still lacks a full `0x6B` modal render and modal action routing.

| Rust area | Status vs this slice | Evidence |
|---|---|---|
| Modal state | `ChooseMapModalState` stores saved selection, selected mode, filtered records, highlighted row, and accept/cancel split | `src/ui/skirmish_shell/state.rs:110` |
| Highlight behavior | `select_map_filtered_row` only changes `highlighted_filtered_index`; it does not commit `selected_map_idx` | `src/ui/skirmish_shell/state.rs:166` |
| Mode change | `select_mode` rebuilds filtered records and resets map top index | `src/ui/skirmish_shell/state.rs:151` |
| Parent action routing | `ChooseMap` still bubbles/swallowed; no full modal screen transition | `src/app.rs:591` |
| Setup preview texture | Preview texture is keyed to committed `selected_map_idx`, not chooser highlight | `src/app_skirmish_shell_render.rs:1705`, `src/app_skirmish_shell_render.rs:1734` |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `0x005E6920` function boundary | verified via label/disassembly | `0x005E68A0` references `LAB_005E6920`; bytes/disassembly `0x005E6920..0x005E7041` | no mutation needed |
| Modal activity in offline YR | verified | `0x005E68A0`, parent reports for `0x006ACEE0` | none |
| `WM_PAINT` preview path | verified | `0x005E696B..0x005E699F`, `0x00640710` | none for browse timing |
| `0x553` map-list normal selection notification | verified negative | `0x005E69B7..0x005E69E3`, `0x005E6B78..0x005E6B96` | row owner-draw visuals out-of-scope |
| `0x6EB` category selection branch | verified | `0x005E6B78..0x005E6DB4` plus helper decompiles | row visual paint out-of-scope |
| `0x497` init branch | verified for preview-negative effects | `0x005E6EA6..0x005E7031` | full list population already covered by sibling reports |
| Use Map commit `0x6C5` | verified | `0x005E6B63..0x005E6B75`, `0x005E7160` | parent return refresh owned by prior report |
| Cancel `0x5C0` | verified | `0x005E69D3..0x005E69FA` | none for preview browsing |
| Create Random Map `0x583` | touched-not-exhausted | `0x005E69FD..0x005E6B57` | full random generation internals out-of-scope |
| Preview loader `0x005E74E0` | verified as absent from normal browse, present in commit/random paths | decompile `0x005E74E0`; branch disassembly | PreviewPack internals out-of-scope |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-01 - Is `0x005E6920` live for standard offline YR Choose Map? -> Yes, wrapper `0x005E68A0` passes label `0x005E6920` to dialog resource `0x6B` and pumps it.` (evidence: `0x005E68A0`)
- `[RESOLVED] OQ-02 - What does modal `WM_PAINT` do with the preview? -> It calls `DrawStartPositions` only if `DAT_00AC1154 != 0`, then validates the dialog.` (evidence: `0x005E696B..0x005E699F`, `0x00640710`)
- `[RESOLVED] OQ-03 - Does `WM_PAINT` read the highlighted `0x553` row? -> No, the paint branch has no `0x553` listbox read and consumes only the current preview wrapper.` (evidence: `0x005E696B..0x005E699F`)
- `[RESOLVED] OQ-04 - Does map-list `0x553` selection change have a command branch? -> No, `WM_COMMAND` handles `0x5C0`, `0x583`, `0x6C5`, and `0x6EB`; other ids fall through.` (evidence: `0x005E69B7..0x005E69E3`, `0x005E6B78..0x005E6B96`)
- `[RESOLVED] OQ-05 - Does category-list `0x6EB` selection change reload the preview? -> No; it rebuilds `0x553`, enables `0x583`, and reselects a row, with no preview loader/invalidation call.` (evidence: `0x005E6B78..0x005E6DB4`)
- `[RESOLVED] OQ-06 - Does init `0x497` reload preview? -> No; it populates list controls, selects committed `DAT_00A8B254`, and clears the init guard.` (evidence: `0x005E6EA6..0x005E7031`)
- `[RESOLVED] OQ-07 - Where does commit happen? -> Use Map `0x6C5` calls `0x005E7160`, which reads `0x553` selected item data, writes selected globals, and closes modal.` (evidence: `0x005E6B63..0x005E6B75`, `0x005E7160`)
- `[RESOLVED] OQ-08 - Does `0x005E7160` itself refresh the preview? -> No; it updates selection globals and text controls, while parent return path later refreshes preview.` (evidence: `0x005E7160`; prior parent refresh report)
- `[RESOLVED] OQ-09 - Is Create Random Map passive browsing? -> No; it is command `0x583`, a side-effect path that can call selected-record/preview helpers.` (evidence: `0x005E69FD..0x005E6B57`)
- `[RESOLVED] OQ-10 - What is the TS legacy status? -> No TS-only gate was found on this UI route; branches are standard shell/modal UI, with random-map path conditional on button/mode state.` (evidence: `0x005E68A0`, `0x005E6920` disassembly)
- `[DEFERRED] OQ-11 - Exact owner-draw row fill/text pixels for listboxes `0x553`/`0x6EB`.` (category: out-of-scope; reason: assigned to row-paint swarm slot; next-step-if-pursued: trace `OwnerDraw_ListBox_00618D40`)
- `[DEFERRED] OQ-12 - Full random-map generation internals after command `0x583`.` (category: out-of-scope; reason: this slice only distinguishes command path from passive browsing; next-step-if-pursued: investigate `0x005E8590`)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Highlighting/browsing map rows in `0x553` does not refresh preview `0x468`; modal paint keeps drawing current `DAT_00AC1154` | `0x005E696B..0x005E699F`; no `0x553` command branch at `0x005E69B7..0x005E6B96` | partial/missing modal render; current preview cache is committed-selection keyed | `src/ui/skirmish_shell/state.rs`, `src/app_skirmish_shell_render.rs`, future `0x6B` modal renderer | Keep chooser highlight separate from committed preview; do not decode/change preview texture on passive row hover/selection | `choose_map_modal_highlight_does_not_change_preview_before_accept` | Do not make the preview feel "helpfully live" while browsing; that would visibly differ from YR |
| Category `0x6EB` selection change rebuilds and reselects `0x553` but still does not reload the preview | `0x005E6B78..0x005E6DB4`; helper decompiles `0x005D63E0`, `0x005EEE40` | `ChooseMapModalState::select_mode` rebuilds list; preview behavior not yet modal-rendered | `src/ui/skirmish_shell/state.rs`, future modal action handlers | Refilter records and select a row without invalidating committed preview until accept or random-map command path | `choose_map_modal_mode_change_preserves_committed_preview_until_accept` | Do not tie filtered-list rebuild to `ensure_selected_preview_texture` |
| Use Map `0x6C5` is the normal commit boundary; preview refresh belongs to parent return path, not transient modal browse | `0x005E6B63..0x005E6B75`, `0x005E7160`, prior `0x006ACEE0` parent refresh report | app currently swallows `ChooseMap`; accept flow not integrated | `src/app.rs`, `src/ui/skirmish_shell/state.rs`, `src/app_skirmish_shell_render.rs` | On accept, commit selection, close modal, then refresh committed setup preview/cache through the parent/setup state path | `choose_map_modal_accept_refreshes_preview_after_commit` | Do not commit selection or refresh preview on list highlight alone |

## Negative Facts / Do Not Do

- Do not update preview `0x468` from passive map-list `0x553` highlight/selection. Active in YR: No; evidence is absence of a `0x553` handler in the active `WM_COMMAND` dispatch.
- Do not treat category `0x6EB` selection change as a preview reload. Active in YR: No for preview refresh; it only rebuilds/selects list rows and button state.
- Do not implement preview `0x468` as a self-painting static in the chooser. Active in YR: No; modal parent `WM_PAINT` calls `DrawStartPositions`, which uses child `0x468` as an anchor.
- Do not use Create Random Map `0x583` as evidence for live browsing preview. Active in YR: Conditional command path only, not row browsing.
- Do not refresh the parent setup preview before modal commit. Active in YR: No for normal browsing; parent refresh occurs after modal return on accept/cancel paths.

## Stale Docs / Follow-up Docs

Replace any wording that says or implies "the Choose Map dialog preview updates as the highlighted row changes" with: "Dialog `0x6B` repaints preview `0x468` from the current global preview wrapper `DAT_00AC1154`; passive `0x553` row highlighting has no preview-refresh branch. The normal preview replacement occurs after Use Map commits and the parent `0x102` return path refreshes the selected map."

Replace any wording that conflates parent setup preview timing with chooser-modal preview timing with: "Parent `0x102` refresh after modal return and chooser `0x6B` browsing are separate: the parent accepted/cancel path reloads/invalidates preview, while the chooser's normal list browsing keeps drawing the pre-existing preview object."

## Sources

- Ghidra read-only decompile: `0x005E68A0`, `0x005E7160`, `0x005E70D0`, `0x005E74E0`, `0x00640710`, `0x005D63E0`, `0x005EEE40`, `0x005EED00`, `0x007CA5D3`, `0x007CA489`.
- Ghidra/read-only byte inspection plus local disassembly of retail bytes: `0x005E6920..0x005E7041`, especially `0x005E696B..0x005E699F`, `0x005E69B7..0x005E6B96`, `0x005E6B78..0x005E6DB4`, `0x005E6EA6..0x005E7031`.
- Prior docs referenced: `SKIRMISH_CHOOSE_MAP_PREVIEW_REFRESH_FUN_006ACEE0_GHIDRA_REPORT.md`, `SKIRMISH_CHOOSE_MAP_PREVIEW_INVALIDATION_GHIDRA_REPORT.md`, `SKIRMISH_CHOOSE_MAP_ACCEPT_CANCEL_SIDE_EFFECTS_GHIDRA_REPORT.md`, `SKIRMISH_CHOOSE_MAP_MODAL_FLOW_GHIDRA_REPORT.md`.
- Rust contrast scan: `src/ui/skirmish_shell/state.rs`, `src/app.rs`, `src/app_skirmish_shell_render.rs`.
