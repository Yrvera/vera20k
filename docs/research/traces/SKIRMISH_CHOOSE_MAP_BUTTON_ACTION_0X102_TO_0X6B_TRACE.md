# Skirmish Choose Map Button Action 0x102 To 0x6B Trace

**Date:** 2026-05-22  
**Scenario:** In the native/dev Skirmish shell dialog `0x102` at `800x600`, click the `Choose Map` button and trace the transition into the map selection/customize battle modal `0x6B`.  
**Scope:** Action generation, app-level state transition, modal opening, first-paint ownership, active modal assets, and return/cancel behavior for this one button path.  
**Non-scope:** Exact listbox row pixel styling, random-map generator UI after `0x583`, broad Skirmish shell visual parity, and live map-preview refresh while browsing inside the chooser.  
**Status:** COMPLETE

## Current Rust Status Correction - 2026-05-23

The current-Rust verdicts in this trace are superseded by
`skirmish-ui/SKIRMISH_CHOOSE_MAP_0X6B_CURRENT_MODAL_RECHECK_GHIDRA_REPORT.md`.
Current Rust now routes `SkirmishShellAction::ChooseMap` into
`open_choose_map_modal`, stores `ChooseMapModalState`, handles Use Map,
Cancel, and list clicks in `handle_choose_map_modal_mouse_down`, and has
saved-selection accept/cancel helpers. The remaining deltas are narrower:
Rust still does not hide/suppress the parent setup rendering like gamemd,
does not draw `MnScrnLCustomizeBattle` modal assets, has non-resource
button/title/preview geometry, leaves Create Random Map as log-only, and
lacks the full parent return/load-failure/row-rebuild contract.

## Pipeline

`left mouse down/up on 0x102 Choose Map -> owner-draw button hit/release -> ChooseMap action/WM_COMMAND 0x5AA -> parent setup hides -> modal wrapper creates dialog 0x6B -> modal first paint uses 0x6B shell assets/controls -> modal result returns -> parent restores/cancels or commits selection and repaints`

## Stage Verdicts

| Stage | Verdict | Rust surface | gamemd evidence | Notes |
|---|---|---|---|---|
| Concrete button identity/action id | PASS | `src/ui/skirmish_shell/state.rs:37`, `:44`, `:1259`, `:1267`, `:1284` | `FUN_006ACEE0 @ 0x006ACEE0`; active branch `param_2 == 0x5AA`; standard `0x102` proc routes `WM_COMMAND` to `FUN_006ACEE0` in `FUN_006AE3F0` | Rust has `OwnerDrawButton::ChooseMap0x5aa` and maps it to `SkirmishShellAction::ChooseMap`. gamemd uses command id `0x5AA` for the same setup button. Numeric command identity matches. |
| Exact click rectangle equality at `800x600` | UNCHECKED | `src/ui/skirmish_shell/layout.rs:180`, `:192`, `:360`, `:446` | Resource `0x102` button id `0x5AA` is active in standard YR, but this trace did not compute the live USER32 pixel child rect after dialog-unit conversion from gamemd runtime state | Rust computes the Choose Map button as `RectPx { x: 635, y: 286, w: 162, h: 37 }` at `800x600`; a click at `(636,287)` is inside. Because the gamemd runtime pixel rect was not computed in this trace, this is not a PASS. |
| Mouse release dispatch into app action handler | FAIL | `src/app.rs:624`, `:629`, `:631`, `:634`, `:635` | `FUN_006AE3F0` routes `WM_COMMAND (0x111)` to `FUN_006ACEE0`; `FUN_006ACEE0` enters the `0x5AA` Choose Map branch | Rust correctly reaches `handle_skirmish_shell_action` with `SkirmishShellAction::ChooseMap`, but that handler later drops the action. gamemd's command branch continues into modal setup. |
| App-level transition from setup `0x102` into chooser `0x6B` | SUPERSEDED: PARTIAL | `src/app.rs::open_choose_map_modal`, `handle_choose_map_modal_mouse_down`; `src/ui/skirmish_shell/state.rs::ChooseMapModalState` | Read-only Ghidra decompile `FUN_006ACEE0`: on `param_2 == 0x5AA`, it saves selected globals, calls `ShowWindow(param_1,0)`, then calls `FUN_005e68a0()` | Current Rust now opens `ChooseMapModalState` and handles modal clicks. Remaining mismatch: gamemd hides setup and owns a separate `0x6B` shell dialog, while Rust overlays a primitive modal over the setup shell. |
| Modal `0x6B` creation and modal pump | SUPERSEDED: PARTIAL | `src/ui/skirmish_shell/state.rs::ChooseMapModalState`; `src/app.rs::open_choose_map_modal`; `src/app_skirmish_shell_render.rs` modal draw path | Read-only Ghidra decompile `FUN_005e68a0`: calls `FUN_0072d120`, creates dialog `0x6B` with callback `LAB_005e6920`, sends `0x4A9`, shows it, then pumps via `FUN_007759e0(0,1,1)` | Current Rust has active modal state and a primitive renderer. Remaining mismatch: it is not a separate native-style `0x6B` dialog with the verified shell assets, owner-draw controls, and exact return/refresh contract. |
| First-paint ownership while chooser is active | FAIL | `src/app.rs:1290`, `:1292`; `src/app_skirmish_shell_render.rs:1806`, `:1823`, `:1862`, `:1909` | `SKIRMISH_CHOOSE_MAP_MODAL_VISUAL_CONTROL_LAYOUT_GHIDRA_REPORT.md`; read-only Ghidra decompile confirms `FUN_006ACEE0` hides setup and `FUN_005e68a0` shows the chooser | gamemd hides the setup dialog and paints the `0x6B` chooser as the active shell modal. Rust continues to render only the base setup shell; there is no branch that replaces it with modal paint. |
| Active modal background/palette asset usage | FAIL | `src/render/skirmish_shell_chrome.rs:789`, `:794`; `src/app_skirmish_shell_render.rs:1178`, `:1191`, `:1200` | `0x0072D120` loads `MnScrnLCustomizeBattle.shp/.PAL` for dialog `0x6B`; `0x0060CF00` binds `DAT_00B0FAB8`/`FUN_0072D210()` for dialog id `0x6B`; active in standard YR through `0x5AA -> FUN_005e68a0` | Rust still classifies `MnScrnLCustomizeBattle.shp` as `ResearchCandidate` and the renderer uses the base `0x102` shell atlas/control draw path only. Player-visible result: the verified chooser background is never drawn. |
| Modal control inventory and first interactive surface | SUPERSEDED: PARTIAL | `src/ui/skirmish_shell/layout.rs::compute_choose_map_modal_layout`; `src/app_skirmish_shell_render.rs` modal overlay/list/text path | Resource `RT_DIALOG 0x6B`: listboxes `0x6EB`/`0x553`, buttons `0x6C5`/`0x583`/`0x5C0`, title `0x694`, status `0x695`, preview `0x468`; active by `FUN_005e68a0` | Current Rust renders a primitive modal/list surface and has active hit testing. Remaining mismatch: button/title/preview/status rects are not the resource `0x6B` rects and row/button paint is not the owner-draw shell art path. |
| Cancel/restore return result `2` | NOT-IMPLEMENTED | `src/ui/skirmish_shell/state.rs:165`, `:172`; no app route in `src/app.rs:585` | `FUN_006ACEE0` compares modal return against `2`; return `2` restores saved selected globals and shows setup again. `0x5C0` Cancel in `0x6B` closes with result `2` per modal layout report | Rust has `cancel_selection()` as an isolated state helper, but the real click never opens modal state, never receives result `2`, and never blocks/restores setup via an app-level modal result. |
| Accepted Use Map return/commit and parent refresh | NOT-IMPLEMENTED | `src/ui/skirmish_shell/state.rs:165`; `src/app.rs:585`; `src/app_skirmish_shell_render.rs:1831` | Non-`2` return from `FUN_005e68a0` rebuilds mode/map state, calls text refresh helpers, refreshes preview object, and invalidates/repaints setup; active in standard YR after accepted modal return | Rust has `accept_selection()` but no route from a modal Use Map button to parent selection commit, text refresh, preview replacement, or setup repaint. |
| Live preview update while browsing inside `0x6B` | UNCHECKED | `src/app_skirmish_shell_render.rs:1831`; no modal preview renderer found | Resource control `0x468` exists in `0x6B`; prior modal report defers exact live browsing-time preview paint | The control exists in gamemd, but this trace did not prove whether row highlight changes repaint the modal preview before Use Map. Rust has no modal preview paint either, but exact gamemd timing is unchecked. |

## Evidence Notes

- Ghidra MCP use in this trace was read-only: decompile and assembly-context only; no renames, labels, comments, program saves, or other mutations.
- Standard YR activity was confirmed through the live offline Skirmish path: `FUN_006AE2C0` creates/pumps dialog `0x102`; `FUN_006AE3F0` routes `WM_COMMAND` into `FUN_006ACEE0`; `FUN_006ACEE0` has the active `0x5AA` branch; that branch calls modal wrapper `FUN_005e68a0`.
- The modal wrapper was spot-checked directly: `FUN_005e68a0` calls `FUN_0072d120`, creates dialog `0x6B` with callback `LAB_005e6920`, sends `0x4A9`, calls `ShowWindow(...,1)`, pumps with `FUN_007759e0(0,1,1)`, and cleans up via `FUN_0072d170`.
- Existing verified reports reconciled: `SKIRMISH_CHOOSE_MAP_MODAL_FLOW_GHIDRA_REPORT.md`, `SKIRMISH_CHOOSE_MAP_PREVIEW_REFRESH_FUN_006ACEE0_GHIDRA_REPORT.md`, `SKIRMISH_CHOOSE_MAP_MODAL_VISUAL_CONTROL_LAYOUT_GHIDRA_REPORT.md`, and `SKIRMISH_CHOOSE_MAP_MODAL_0X6B_VISUAL_INTEGRATION_GHIDRA_REPORT.md`.

## Failures

- The click generates a `ChooseMap` action, but `src/app.rs` drops it. The player clicks the button and remains on the base setup shell instead of seeing a chooser.
- The first-paint owner is wrong after the click. gamemd hides setup `0x102` and shows modal `0x6B`; Rust keeps rendering setup `0x102`.
- The active modal asset path is wrong/missing. gamemd uses `MnScrnLCustomizeBattle.shp/.PAL` for `0x6B`; Rust does not expose or draw it as a modal background.

## Not Implemented

- App-level active chooser/modal state for dialog `0x6B`.
- Modal render path for background, two listboxes, right-column buttons, title/status statics, and preview control.
- Modal result handling for Cancel result `2`, accepted Use Map, parent setup restoration, text refresh, selected-map commit, and preview refresh.

## Adjacent Findings

- `ChooseMapModalState` and `compute_choose_map_modal_layout` are useful scaffolding, but they are currently unreachable from the actual button click.
- `MnScrnLCustomizeBattle.shp/.PAL` should remain excluded from base setup `0x102`; it is verified for the chooser `0x6B`.
- Exact listbox owner-draw row styling and live browsing-time preview updates remain focused follow-ups, not blockers to proving this button transition is missing.

## Verification

No Rust code was changed and no cargo tests were run. This was a read-only trace plus one report write.

## Verdict Tally

PASS: 1 | FAIL: 3 | UNCHECKED: 2 | NOT-IMPLEMENTED: 4
