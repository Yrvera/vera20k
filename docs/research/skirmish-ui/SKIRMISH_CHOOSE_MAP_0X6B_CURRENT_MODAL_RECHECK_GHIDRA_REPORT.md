# Skirmish Choose Map 0x6B Current Modal Recheck - Ghidra Research Report

**Address(es):** `0x006ACEE0`, `0x005E68A0`, callback entry bytes at `0x005E6920`, `0x005E7160`, `0x0060CF00`, `0x0072D120`, `0x00622820`, `0x0060F9A0`
**Investigation Mode:** coverage-map scoped to current Rust reconciliation and stale trace replacement.
**Target question:** Reconcile current Rust Choose Map modal open/render/input/return path against active gamemd.exe dialog `0x6B` behavior, especially claims made obsolete by `ChooseMapModalState` and app-level handlers.
**Non-goals:** Random map generator internals after `0x583`, full scenario source census, broad `0x102` shell parity, and re-investigating settled list population/preview decode except where needed to compare current Rust.
**Evidence needed to mark COMPLETE:** Fresh current Rust scan, prior Choose Map reports reconciled, read-only Ghidra spot-checks for the active `0x5AA -> 0x6B` path and handoff-critical command/result/asset/listbox facts, exact stale-doc replacement wording, and at least one Rust-facing implementation handoff item.
**Stop conditions:** Write exactly this report plus the shared claims file, no Ghidra mutations, no Rust edits, no INI edits, and unresolved items listed as Remaining Uncertainty.
**Claimed Scope:** Standard offline YR Skirmish setup button `0x5AA` through modal dialog `0x6B` open, active paint/input ownership, Use Map/Cancel/Create Random Map boundary, current Rust modal implementation status, and stale trace replacement wording.
**Non-Scope:** Downstream random-map creation UI/generator, full row-paint rediscovery, runtime screenshot RGB validation, and non-offline/WOL variants.
**Confidence:** High for modal liveness, app open status, accept/cancel command result, distinct modal asset binding, owner-draw class identity, and current Rust contrast. Medium for exact first-frame pixel parity because runtime screenshots were not captured and `0x005E6920` has no Ghidra function boundary in this read-only session.
**Active in YR:** Yes for standard offline Skirmish; Conditional for branches gated by player clicking `0x6C5`, `0x5C0`, or `0x583`, and for the exact 800-wide alternate SHP load.

## 1. Overview

Older traces that say Rust drops `ChooseMap` or has no modal render path are stale. Current Rust now opens `ChooseMapModalState` from `src/app.rs`, handles modal list/button clicks, and draws a primitive modal overlay with text/list rows.

The remaining player-visible mismatch is still large but narrower: gamemd hides the parent setup dialog and shows a separate `0x6B` shell dialog using `MnScrnLCustomizeBattle` assets and real owner-drawn controls. Current Rust draws the base setup shell first, then overlays primitive solid rectangles; its `0x6B` button/title/preview geometry does not match the resource contract; `Create Random Map` only logs; and accepted/cancel preview/row-rebuild semantics are only partially modeled.

## 2. Class Layout / Key Offsets

| Item | Role | Evidence | Active in YR |
|---|---|---|---|
| `DAT_00A8B250` | selected mode/category token saved before modal and restored/committed after return | `0x006ACEE0` decompile; `0x005E7376` writes on accept | Yes |
| `DAT_00A8B254` | selected scenario index saved before modal and restored/committed after return | `0x006ACEE0` decompile; `0x005E7370` and `0x005E7388` writes | Yes |
| `DAT_00AC0D40` | active Choose Map modal HWND | `0x005E68D0` writes returned HWND | Yes |
| shell parent record `+0x74` | convert/palette pointer chosen by dialog id | `0x0060CF00` decompile writes `FUN_0072D210()` for id `0x6B` | Yes |
| shell parent record `+0xE0` / `+0xE4` | small/alternate background SHP pointers | `0x0060CF00` decompile writes `DAT_00B0FB50` and `DAT_00B0FAB8` for id `0x6B` | Yes |

Rust contrast, not binary evidence: current Rust stores modal state in `SkirmishShellState.choose_map_modal` and `ChooseMapModalState` fields `saved_selection`, `selected_mode_id`, `filtered_record_indices`, `highlighted_filtered_index`, `mode_top_index`, and `map_top_index` (`src/ui/skirmish_shell/state.rs:128`). Active in YR: n/a, this is implementation status.

## 3. Core Logic

### 3.1 Parent button opens a real modal

Finding: Standard offline Skirmish `0x5AA` saves current selection, hides setup, calls the modal wrapper, then branches on the modal result.

Evidence: `0x006ACEE0` decompile shows the `param_2 == 0x5AA` branch copying `DAT_00A8B8E0`, calling `FUN_00608070`, `ShowWindow(param_1,0)`, `FUN_005E68A0()`, and comparing the return with literal `2`.

Active in YR: Yes. Prior reports verify `0x006AE2C0` creates offline dialog `0x102` and `0x006AE3F0` routes `WM_COMMAND` to `0x006ACEE0`; fresh decompile confirms the live `0x5AA` branch.

Current Rust status: now partially matches entry. `src/app.rs:687` routes `SkirmishShellAction::ChooseMap` to `open_choose_map_modal`, which initializes `ChooseMapModalState` at `src/app.rs:696`. Older "app swallows ChooseMap" claims are stale.

### 3.2 Modal wrapper and modal ownership

Finding: `0x005E68A0` creates dialog id `0x6B`, stores its HWND, sends init message `0x4A9`, shows the chooser, pumps a modal loop, then cleans up modal assets.

Evidence: decompile of `0x005E68A0`; assembly context `0x005E68B7 MOV EDX,0x6B`, `0x005E68BE PUSH 0x5E6920`, `0x005E68C4 CALL 0x00775700`, `0x005E68D0 MOV [0x00AC0D40],EAX`, `0x005E68E3 PUSH 0x4A9`, `0x005E68F5 PUSH 0x1`, `0x005E690F CALL 0x007759E0`.

Active in YR: Yes.

Current Rust delta: modal state exists, but render ownership does not match. `src/app_skirmish_shell_render.rs:2256` computes a `choose_map_layout`, then still builds base setup instances and text (`src/app_skirmish_shell_render.rs:2316`, `src/app_skirmish_shell_render.rs:2326`) before adding modal overlay. That keeps setup visuals under the chooser; gamemd hides setup.

### 3.3 Dialog `0x6B` asset binding

Finding: Choose Map `0x6B` has a distinct modal background/palette path. It is not the base `0x102` setup background.

Evidence: `0x0072D120` decompile loads modal assets once. Assembly `0x0072D129 CMP [g_ScreenWidth],0x320` gates `DAT_00B0FAB8` SHP load at exactly 800 width; `0x0072D14A..0x0072D15A` loads PAL/convert state via pointer table. `0x0060CF00` decompile has a distinct `iVar3 == 0x6B` branch writing `FUN_0072D210()`, `DAT_00B0FB50`, and `DAT_00B0FAB8`; the `0x102` branch writes `FUN_0072D030()`, `DAT_00B0FB50`, and `DAT_00B0FA18`.

Active in YR: Yes for `0x6B`; Conditional for `DAT_00B0FAB8` SHP load because the binary checks screen width equals 800, not greater-than-or-equal.

Current Rust delta: missing. `src/render/skirmish_shell_chrome.rs:166` packs only `MNSCRNS.SHP` and `MnScrnLCoopGameSetup.shp` using `MnScrnLCoopGameSetup.PAL`; `src/render/skirmish_shell_chrome.rs:794` still classifies `MnScrnLCustomizeBattle.shp` as `ResearchCandidate`. Modal drawing uses solid colors `SHELL_MODAL_BG_RGB` / `SHELL_MODAL_PANEL_RGB` (`src/app_skirmish_shell_render.rs:67`, `src/app_skirmish_shell_render.rs:1118`).

### 3.4 Control inventory and geometry

Finding: The modal resource is a `533x369` dialog with listboxes `0x6EB` and `0x553`, buttons `0x6C5`, `0x583`, `0x5C0`, title static `0x694`, preview static `0x468`, status static `0x695`, and heading/static labels. The right buttons are resource-local `UseMap=(425,122,108,23)`, `CreateRandomMap=(425,149,108,23)`, `Cancel=(425,346,108,23)`.

Evidence: `SKIRMISH_CHOOSE_MAP_MODAL_VISUAL_CONTROL_LAYOUT_GHIDRA_REPORT.md` retail `RT_DIALOG 0x6B` extraction; fresh Ghidra context confirms modal list population/command sites (`0x005E6EA6`, `0x005E6F17`, `0x005E69C2..0x005E69EC`). Handoff-critical button command order is also supported by callback bytes: `0x005E69C2 CMP EAX,0x6C5`, `0x005E69D3 SUB EAX,0x583`, then cancel result `2` at `0x005E69E7`.

Active in YR: Yes for the dialog/control inventory; Conditional for `0x583` behavior because it requires the player to click Create Random Map.

Current Rust delta: mixed. `src/ui/skirmish_shell/layout.rs:28` and `:29` use the correct `533x369` modal size, and listbox rects match previous verified local coordinates. But `src/ui/skirmish_shell/layout.rs:531` models title as local `(0,20,533,24)` instead of resource `0x694=(425,1,108,10)`, `src/ui/skirmish_shell/layout.rs:532` models preview as local `(374,202,128,96)` instead of resource `0x468=(428,23,96,69)`, and button rects at `src/ui/skirmish_shell/layout.rs:527..530` remain non-resource-sized/reordered compared with `0x6B`.

### 3.5 Listbox and button classes

Finding: `0x6EB` and `0x553` are real owner-drawn `LISTBOX` controls; modal buttons are owner-drawn shell push buttons.

Evidence: resource style/class from `SKIRMISH_CHOOSE_MAP_MODAL_VISUAL_CONTROL_LAYOUT_GHIDRA_REPORT.md`; `0x0060F9A0` decompile maps class `"ListBox"` to `OwnerDraw_ListBox_00618D40` at the `0x0060FC18` branch and class `"Button"` with `(style & 0x0B) == 0x0B` to `OwnerDraw_Button_00612B70` at the `0x0060FE58` branch. `0x005E7160` uses `LB_GETCURSEL 0x188` and `LB_GETITEMDATA 0x199` on `0x553` and `0x6EB`.

Active in YR: Yes.

Current Rust delta: partial. `CHOOSE_MAP_LIST_ROW_H` is now `19` (`src/ui/skirmish_shell/layout.rs:30`), matching the sibling listbox row formula. Current modal rendering still uses primitive rectangles/outlines and text draws (`src/app_skirmish_shell_render.rs:1118`, `src/app_skirmish_shell_render.rs:1935`), not `OwnerDraw_ListBox_00618D40` row fill/scrollbar art or shell button PCX pieces for modal buttons.

### 3.6 Use Map, Cancel, and Create Random Map side effects

Finding: Use Map commits by reading selected item data from `0x553` and selected mode item data from `0x6EB`; Cancel closes with modal result `2`; Create Random Map is a separate branch, not a no-op.

Evidence: `0x005E7160` decompile reads `SendDlgItemMessageA(DAT_00AC0D40,0x553,0x188/0x199,...)`, resolves the record in `DAT_00A8B8CC`, reads `0x6EB`, writes `DAT_00A8B23C`, `DAT_00A8B254`, and `DAT_00A8B250`, then closes with result `1` at `0x005E73A8..0x005E73AD`. Callback context `0x005E69E7 MOV EDX,0x2` closes Cancel with result `2`. `0x005E69D3` identifies the `0x583` branch before the cancel check.

Active in YR: Yes for Use Map and Cancel; Conditional for Create Random Map when clicked.

Current Rust delta: partial. `src/app.rs:765` handles modal mouse-down, `src/app.rs:817` commits Use Map and `src/app.rs:789` recognizes `CreateRandomMap0x583`, but the random-map branch only logs. `commit_choose_map_selection` directly updates Rust selected mode/map and clears preview texture (`src/app.rs:731`) without the verified parent-side row rebuild/load-failure restoration order from `0x006ACEE0`.

## 4. INI Keys

No INI key is directly read in this scoped modal open/render/input path. `MPModesMD.ini` and scenario records determine list content through already-settled sibling reports, but this recheck did not reopen MPModes parsing or random map generation internals.

Active in YR: n/a for direct INI reads in this slice.

## 5. Integration Points

| Integration point | Behavior | Evidence | Active in YR |
|---|---|---|---|
| `0x102` command route | `WM_COMMAND` low word reaches `0x006ACEE0`; `0x5AA` enters Choose Map branch | prior `0x006AE3F0` reports plus fresh `0x006ACEE0` decompile | Yes |
| modal creation | `0x005E68A0` creates resource `0x6B`, sends `0x4A9`, shows, pumps | fresh decompile and assembly context | Yes |
| common shell setup | `0x00622820` enumerates children, installs owner-draw hooks, marks fullscreen shell, moves parent to full screen for ids including `0x6B` | fresh `0x00622820` decompile | Yes |
| asset binding | `0x0060CF00` chooses `0x6B` background/palette fields | fresh `0x0060CF00` decompile | Yes |
| accept/cancel | `0x005E7160` closes with result `1`; cancel closes with result `2` | fresh `0x005E7160` decompile and callback assembly context | Yes / Conditional by clicked button |

## 6. Current Rust Implementation Status

| Rust surface | Status | Evidence |
|---|---|---|
| App open path | implemented | `src/app.rs:687` calls `open_choose_map_modal`; `src/app.rs:696` initializes state |
| Modal state/filter/accept/cancel helpers | implemented/partial | `src/ui/skirmish_shell/state.rs:128`, `:138`, `:191`, `:198`, `:202`, `:218` |
| Modal list/button hit testing | implemented/partial | `src/app.rs:765`; layout hit tests at `src/ui/skirmish_shell/layout.rs:536` and `:553` |
| Modal render path | partial primitive overlay | `src/app_skirmish_shell_render.rs:1118`, `:1494`, `:1935`, `:2326` |
| Modal background assets | missing | `src/render/skirmish_shell_chrome.rs:166`, `:794` |
| Parent hide/show ownership | mismatch | renderer still draws base setup before modal overlay (`src/app_skirmish_shell_render.rs:2316`, `:2326`) |
| Resource-accurate statics/buttons/preview | mismatch | layout title/preview/buttons at `src/ui/skirmish_shell/layout.rs:527..532` do not match `0x6B` resource |
| Random map button | recognized only | `src/app.rs:789` logs instead of entering verified random-map flow |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Parent `0x5AA` modal branch | verified | `0x006ACEE0` decompile | none for open/return split |
| Modal wrapper `0x005E68A0` | verified | decompile plus assembly `0x005E68B7..0x005E690F` | none for wrapper |
| Callback boundary at `0x005E6920` | touched-not-exhausted | disassembly/context only; no function boundary found read-only | full callback decompile would require boundary repair or deeper disassembly pass |
| Use Map commit helper `0x005E7160` | verified | decompile plus assembly `0x005E7367..0x005E73AD` | none for accept result/selected globals |
| Cancel result `2` | verified | assembly context `0x005E69D3..0x005E69EC` | none |
| Create Random Map button boundary | touched-not-exhausted | assembly context identifies `0x583` branch | downstream random-map flow out of scope |
| `0x6B` asset binding | verified | `0x0072D120`, `0x0060CF00` decompile/context | runtime screenshot for `>800` fallback |
| Owner-draw class mapping | verified | `0x0060F9A0` decompile | exact modal first-frame pixels need screenshot |
| Current Rust app state | verified | code scan `src/app.rs`, `src/ui/skirmish_shell/state.rs` | implementation |
| Current Rust render/layout | verified | code scan `src/app_skirmish_shell_render.rs`, `src/ui/skirmish_shell/layout.rs`, `src/render/skirmish_shell_chrome.rs` | implementation |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-1 - Does current Rust still drop ChooseMap at app level? -> No; it now calls open_choose_map_modal.` (evidence: `src/app.rs:687`, `src/app.rs:696`)
- `[RESOLVED] OQ-2 - Does current Rust have any modal render path? -> Yes, a primitive overlay/list/text path exists.` (evidence: `src/app_skirmish_shell_render.rs:1118`, `src/app_skirmish_shell_render.rs:1935`, `src/app_skirmish_shell_render.rs:2326`)
- `[RESOLVED] OQ-3 - Does gamemd hide the parent setup before showing dialog 0x6B? -> Yes.` (evidence: `0x006ACEE0` decompile)
- `[RESOLVED] OQ-4 - Is 0x6B asset binding distinct from 0x102? -> Yes; 0x6B binds DAT_00B0FAB8/FUN_0072D210, while 0x102 binds DAT_00B0FA18/FUN_0072D030.` (evidence: `0x0060CF00`)
- `[RESOLVED] OQ-5 - Are 0x6EB and 0x553 combos? -> No, they are ListBox controls and accept reads LB_GETCURSEL/LB_GETITEMDATA.` (evidence: `0x0060F9A0`, `0x005E7160`, resource report)
- `[RESOLVED] OQ-6 - Is Use Map result/argument ordering verified strongly enough for handoff? -> Yes; 0x005E7160 reads map list first, mode list second, writes globals, then calls 0x007757E0 with result 1.` (evidence: `0x005E7160`, assembly `0x005E7367..0x005E73AD`)
- `[RESOLVED] OQ-7 - Is Cancel result verified? -> Yes, callback branch moves EDX=2 before modal close helper.` (evidence: `0x005E69E7`, `0x005E69EC`)
- `[RESOLVED] OQ-8 - Does current Rust button/static geometry match all 0x6B controls? -> No; listbox size matches, but buttons/title/preview/status/heading representation diverges or is incomplete.` (evidence: `src/ui/skirmish_shell/layout.rs:527..532`; resource report)
- `[DEFERRED] OQ-9 - Full decompile of callback entry 0x005E6920.` (category: `requires-different-system-context`; reason: no function boundary found and Ghidra mutation is forbidden in this swarm; next-step-if-pursued: run a read-only disassembly-focused callback report or approve boundary creation in a non-swarm session)
- `[DEFERRED] OQ-10 - Runtime first-frame RGB at 800 and >800 widths.` (category: `needs-runtime-debugger`; reason: this slot did not capture native screenshots; next-step-if-pursued: screenshot native dialog `0x6B` at 800x600 and 1024x768)
- `[DEFERRED] OQ-11 - Downstream Create Random Map UI/generator behavior.` (category: `out-of-scope`; reason: only button boundary was in scope; next-step-if-pursued: use the existing RMG reports)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Proposed test name | Risk / do-not-do |
|---|---|---|---|---|---|---|---|
| Choose Map is a separate modal screen: setup hides before `0x6B` shows, then setup returns after modal result. | `0x006ACEE0`; `0x005E68A0` decompile/context | partial mismatch: modal opens, but renderer draws setup under a primitive overlay | `src/app_skirmish_shell_render.rs`, `src/app.rs` | When `choose_map_modal` is active, suppress base setup control/background composition or otherwise model the separate dialog ownership, then restore setup after result. | Click Choose Map at 800x600: setup controls are not visible/interactive behind the chooser; Cancel returns to unchanged setup. | `choose_map_modal_suppresses_parent_setup_until_result` | Do not keep parent setup as the visual/input owner underneath the chooser. |
| `0x6B` uses `MnScrnLCustomizeBattle.shp/.PAL` binding, with alternate SHP loaded only at exact width 800. | `0x0072D120`, `0x0060CF00`, prior string/resource report | missing: asset remains `ResearchCandidate`; modal uses solid colors | `src/render/skirmish_shell_chrome.rs`, `src/app_skirmish_shell_render.rs` | Pack the modal-specific background/palette path and draw it for `0x6B`; preserve the distinct `0x102` background path. | 800x600 chooser background uses Customize Battle art, not a solid panel and not `MnScrnLCoopGameSetup`. | `choose_map_modal_uses_customize_battle_background_at_800` | Do not reuse `0x102` setup background for the chooser; do not promote `MnScrnLCustomizeBattle` as a base setup asset. |
| `0x6B` button/title/preview/static geometry comes from resource controls; Use Map `(425,122,108,23)`, Create Random `(425,149,108,23)`, Cancel `(425,346,108,23)`, preview `0x468=(428,23,96,69)`. | resource report plus callback command context `0x005E69C2..0x005E69EC` | mismatch: Rust uses non-resource button/title/preview rects and omits some static/status rects | `src/ui/skirmish_shell/layout.rs`, `src/app.rs`, `src/app_skirmish_shell_render.rs` | Align hit tests and rendering to `0x6B` resource rects and include title/headings/status/preview controls. | Clicking the bottom-right Cancel closes; clicking where Rust's old upper Cancel was does not. | `choose_map_modal_buttons_and_statics_match_resource_0x6b_rects` | Do not preserve the current upper-right Cancel layout; it contradicts resource `0x6B`. |
| Use Map commits on accept helper only; Cancel result `2` restores saved selection; passive row highlight is not the committed setup selection. | `0x005E7160`, `0x006ACEE0`, `0x005E69E7` | partial: saved/cancel helper exists, but accept lacks parent row rebuild/load-failure restore order | `src/app.rs`, `src/ui/skirmish_shell/state.rs`, preview cache/session state | Keep transient highlight separate from committed map; on Use Map run the verified commit/refresh boundary; on failed load restore old selection. | Select a different row, Cancel: selected setup map and preview remain old; Use Map commits and refreshes only after close. | `choose_map_modal_cancel_restores_and_use_map_commits_after_close` | Do not update setup preview or launch map from transient highlight alone. |
| `0x583` Create Random Map is an active branch in the modal callback. | callback context `0x005E69D3` identifies `0x583` branch; sibling RMG reports cover downstream behavior | missing: Rust logs only | `src/app.rs`, RMG modal/state surface | Treat button as a real command boundary; wire to verified random-map setup only after applying RMG contracts. | Clicking Create Random Map does not silently do nothing; it enters the verified random-map path or a blocked explicit state. | `choose_map_modal_create_random_map_is_not_noop` | Do not leave this as an invisible log-only action in player builds. |

## 10. Negative Facts / Do Not Do

- Do not keep using older traces that say `ChooseMap` is swallowed at app level. Active in YR: n/a for Rust status; current evidence is `src/app.rs:687` and `src/app.rs:696`.
- Do not say Rust has no modal renderer at all. Active in YR: n/a for Rust status; current evidence is primitive modal rendering at `src/app_skirmish_shell_render.rs:1118` and modal text at `src/app_skirmish_shell_render.rs:1935`.
- Do not treat current Rust primitive overlay as parity-complete. Active in YR: No; gamemd uses a separate shell dialog, modal asset binding, owner-draw listboxes/buttons, and parent hide/show (`0x006ACEE0`, `0x005E68A0`, `0x0060CF00`, `0x0060F9A0`).
- Do not model `0x6EB`/`0x553` as combo dropdowns. Active in YR: No; they are `LISTBOX` controls and `0x005E7160` uses listbox messages.
- Do not make row highlight a committed map or preview refresh boundary. Active in YR: No for normal browsing per `SKIRMISH_CHOOSE_MAP_0X6B_PREVIEW_REFRESH_GHIDRA_REPORT.md`; commit is Use Map/parent return (`0x005E7160`, `0x006ACEE0`).

## 11. Stale Docs / Follow-up Docs

- `docs/research/traces/SKIRMISH_CHOOSE_MAP_BUTTON_ACTION_0X102_TO_0X6B_TRACE.md`: replace the Rust status claims "Mouse release dispatch into app action handler: FAIL", "App-level transition ... NOT-IMPLEMENTED", "Modal 0x6B creation and modal pump: NOT-IMPLEMENTED", and "Cancel/restore return result 2: NOT-IMPLEMENTED" with: "STALE as of 2026-05-23 current Rust. `src/app.rs` now routes `SkirmishShellAction::ChooseMap` into `open_choose_map_modal`, stores `ChooseMapModalState`, handles Use Map/Cancel/list clicks in `handle_choose_map_modal_mouse_down`, and has saved-selection accept/cancel helpers. Remaining deltas: Rust still does not hide/suppress the parent setup rendering like gamemd, does not draw `MnScrnLCustomizeBattle` modal assets, has non-resource button/title/preview geometry, leaves Create Random Map as log-only, and lacks the full parent return/load-failure/row-rebuild contract."
- `docs/research/traces/SKIRMISH_CHOOSE_MAP_MODAL_FIRST_PAINT_VISUAL_TRACE.md`: replace the top "Current Rust reaches only: ChooseMap0x5aa hit-test -> SkirmishShellAction::ChooseMap -> app swallows action -> parent setup remains on screen" with: "STALE as of 2026-05-23 current Rust. Current Rust reaches `ChooseMap0x5aa -> SkirmishShellAction::ChooseMap -> open_choose_map_modal -> ChooseMapModalState`, then renders a primitive modal overlay and text/list rows. The current first-paint mismatch is no longer absence of state/rendering; it is that the parent setup remains drawn underneath, modal background uses primitive solid rectangles instead of `MnScrnLCustomizeBattle` assets, and several `0x6B` resource rects/statics/buttons remain wrong or absent."
- `docs/research/traces/SKIRMISH_CHOOSE_MAP_MODAL_FIRST_PAINT_VISUAL_TRACE.md`: replace the Stage 5 button-geometry claim as still current: "The mismatch remains current: gamemd resource buttons are Use Map `(425,122,108,23)`, Create Random Map `(425,149,108,23)`, and Cancel `(425,346,108,23)`, while current Rust still uses non-resource modal button rects in `compute_choose_map_modal_layout`."

## 12. Remaining Uncertainty

- `0x005E6920` callback could not be decompiled as a function in this read-only swarm because no function boundary exists; command branches were checked through disassembly/context instead.
- Exact runtime first-frame RGB and `>800` modal background fallback need native screenshot validation.
- Downstream Create Random Map UI/generator behavior remains delegated to the existing RMG reports.

## Sources

- Fresh read-only Ghidra: `decompile_function 006acee0`, `decompile_function 005e68a0`, `decompile_function 005e7160`, `decompile_function 0072d120`, `decompile_function 0060cf00`, `decompile_function 00622820`, `decompile_function 0060f9a0`.
- Fresh read-only Ghidra assembly/context: `0x005E68B7..0x005E690F`, `0x005E69C2..0x005E69EC`, `0x005E6B63`, `0x005E6EA6`, `0x005E6F17`, `0x005E7367..0x005E73AD`, `0x0072D129..0x0072D15A`.
- Ghidra read-only limitation observed: `decompile_function 005e6920` returned no function; no function boundary was created.
- Existing docs reconciled: `SKIRMISH_CHOOSE_MAP_MODAL_FLOW_GHIDRA_REPORT.md`, `SKIRMISH_CHOOSE_MAP_MODAL_VISUAL_CONTROL_LAYOUT_GHIDRA_REPORT.md`, `SKIRMISH_CHOOSE_MAP_MODAL_0X6B_VISUAL_INTEGRATION_GHIDRA_REPORT.md`, `SKIRMISH_CHOOSE_MAP_ACCEPT_CANCEL_SIDE_EFFECTS_GHIDRA_REPORT.md`, `SKIRMISH_OWNERDRAW_LISTBOX_00618D40_ROW_PAINT_GHIDRA_REPORT.md`, `SKIRMISH_CHOOSE_MAP_0X6B_PREVIEW_REFRESH_GHIDRA_REPORT.md`.
- Stale docs checked: `docs/research/traces/SKIRMISH_CHOOSE_MAP_BUTTON_ACTION_0X102_TO_0X6B_TRACE.md`, `docs/research/traces/SKIRMISH_CHOOSE_MAP_MODAL_FIRST_PAINT_VISUAL_TRACE.md`.
- Rust scan: `src/app.rs`, `src/ui/skirmish_shell/state.rs`, `src/ui/skirmish_shell/layout.rs`, `src/app_skirmish_shell_render.rs`, `src/render/skirmish_shell_chrome.rs`.
