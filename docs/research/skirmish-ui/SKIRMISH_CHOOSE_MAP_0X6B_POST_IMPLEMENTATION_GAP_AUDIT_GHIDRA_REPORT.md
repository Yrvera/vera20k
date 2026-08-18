# Skirmish Choose Map 0x6B Post-Implementation Gap Audit - Ghidra Research Report

**Address(es):** `0x005E68A0`, `0x005E7160`, `0x0060F9A0`, `0x00612B70`, `0x00618D40`, `0x00622B50`, `0x006040B0`, `0x0060CF00`, `0x0072D120`  
**Investigation Mode:** coverage-map  
**Claimed Scope:** Current Rust Choose Map modal after the native-modal implementation pass: what now matches verified `0x6B` behavior, what still diverges, and which follow-up slices are highest leverage.  
**Non-Scope:** Implementing fixes, full random-map generator internals, runtime screenshot capture, full callback disassembly for `LAB_005E6920`, and non-offline/WOL variants.  
**Confidence:** High for modal ownership, asset binding, resource/control geometry, listbox row basics, button owner-draw contract, and `0x6B` status tooltip mapping. Medium for keyboard/default-button and exact first-frame pixels because this pass did not run native runtime capture or drain the missing callback boundary.  
**Active in YR:** Yes for standard offline Skirmish `Choose Map`; Conditional for Create Random Map and overflow scrollbars.

## 1. Overview

This pass extends existing Choose Map reports rather than redoing them. The prior reports already prove the native model: parent setup hides, modal dialog `0x6B` opens, `MnScrnLCustomizeBattle.*` is the modal asset path, `0x6EB` and `0x553` are real owner-drawn listboxes, row highlighting does not refresh the preview, Use Map commits, and Cancel closes without committing.

Current Rust has moved materially closer since the older current-modal report: it now suppresses parent shell composition while the chooser is active, loads/classifies `MnScrnLCustomizeBattle`, uses the verified title/heading/status/preview rects, uses `19` px list rows, reserves `20` px scrollbar width, and makes Cancel close without committing. The remaining player-visible gaps are now narrower: modal button geometry is still non-resource-sized/reordered, modal button pressed/capture behavior is not modeled, status/help text has no `0x6B` hover mapping, mouse wheel behavior is currently invented/not verified for the native callback, Create Random Map is still log-only, and preview/random-map paths need screenshot/runtime validation.

Prior-state decision: recent high-confidence reports exist, but they have stale Rust-status rows and explicit deferred gaps. This report therefore proceeds as "scope to gaps + verification only."

## 2. Current Rust Delta Snapshot

| Area | Current Rust status | Evidence |
|---|---|---|
| Separate modal state | present | `src/app.rs::open_choose_map_modal`; `ChooseMapModalState` in `src/ui/skirmish_shell/state.rs` |
| Parent shell hidden while modal active | mostly fixed | `build_skirmish_shell_instances` returns modal-only instances when `choose_map_layout` is present |
| `MnScrnLCustomizeBattle` asset | mostly fixed for 800-wide shell | `SkirmishShellChromeAtlas.choose_map_background_800_customize_battle`; `choose_map_background_entry` |
| Resource statics/preview/status rects | mostly fixed | `compute_choose_map_modal_layout` title `(425,1,108,10)`, preview `(428,23,96,69)`, status `(2,355,303,12)` |
| Modal buttons | still divergent | Rust uses local `(374,80/116/152,112,30)` for Use/Cancel/Create, not resource `(425,122,108,23)`, `(425,346,108,23)`, `(425,149,108,23)` |
| Real-listbox row height/content shrink | mostly fixed | `CHOOSE_MAP_LISTBOX_ROW_H = 19`; content rect reserves `20` px when overflowed |
| Scrollbar input | partial/unverified | Rust supports arrows, track clicks, wheel; native listbox/scrollbar reports verify arrows/track/thumb class behavior but not direct wheel handling |
| Passive highlight vs commit | mostly fixed | `select_map_filtered_row` only changes highlight; `commit_choose_map_selection` runs on Use Map |
| Cancel | fixed relative to previous review | `Cancel0x5c0` closes without `cancel_selection()` commit |
| Status/help `0x695` | partial | state/render field exists, but no modal hover mapping to `STT:Scenario*` strings |
| Create Random Map | missing | `CreateRandomMap0x583` logs only |

## 3. Binary Findings

### 3.1 Modal ownership remains the primary screen-state contract

Active in YR: Yes. `FUN_005E68A0` constructs the chooser dialog, loads modal assets through `FUN_0072D120`, creates dialog resource `0x6B` with callback label `LAB_005E6920`, stores the HWND in `DAT_00AC0D40`, sends init message `0x4A9`, shows the chooser, pumps modal loop `0x007759E0`, then frees the modal asset state.

Evidence: fresh decompile of `FUN_005e68a0`.

Implementation consequence: current Rust's modal-only instance path is the right direction. The renderer should not regress to drawing parent `0x102` controls under the chooser.

### 3.2 Modal buttons are still the largest obvious geometry miss

Active in YR: Yes. Resource `0x6B` defines buttons:

| Control | Resource-local rect | Role |
|---|---:|---|
| `0x6C5` | `(425,122,108,23)` | Use Map |
| `0x583` | `(425,149,108,23)` | Create Random Map |
| `0x5C0` | `(425,346,108,23)` | Cancel |

Rust currently uses local `(374,80,112,30)`, `(374,152,112,30)`, and `(374,116,112,30)` respectively. That means the player can click where native has empty right-column background and activate a button, while the native bottom-right Cancel location is not modeled.

Evidence: `SKIRMISH_CHOOSE_MAP_MODAL_VISUAL_CONTROL_LAYOUT_GHIDRA_REPORT.md` resource extraction; fresh current Rust scan of `compute_choose_map_modal_layout`.

### 3.3 Owner-draw button behavior applies to `0x6B` modal buttons too

Active in YR: Yes. `FUN_0060F9A0` maps `BUTTON` style low bits `(style & 0x0B) == 0x0B` to `OwnerDraw_Button_00612B70`. The `0x6B` buttons all use style `0x5000000B`.

Fresh decompile of `OwnerDraw_Button_00612B70` shows:

- `WM_LBUTTONDOWN` / `WM_LBUTTONDBLCLK` play the main button sound unless disabled (`piVar17[0x2F]` gate).
- Paint chooses up/down/disabled art state from owner-draw state bits and `WS_DISABLED`.
- The first up-to-down paint transition can play `GenericClick` when not disabled.
- Text is drawn after the button art via `FUN_00621040`, with a pressed offset.

Current Rust uses `push_button_30` for modal buttons but passes `pressed = false` for all three modal buttons and handles activation directly on mouse down. This is not native-feeling: native owner-draw buttons have down-state capture/paint behavior and activate through the button/control flow, not an immediate no-capture click.

Evidence: fresh decompile `OwnerDraw_Button_00612B70`; `FUN_0060F9A0`; prior `SKIRMISH_OWNERDRAW_PUSH_BUTTON_SOUNDS_GHIDRA_REPORT.md`.

### 3.4 `0x6B` status/help has verified tooltip mappings

Active in YR: Yes. The common parent handler `FUN_00622B50` handles `WM_NCHITTEST`, gets child `0x695`, hit-tests the child under the cursor, asks for a specific tooltip/status string, and sends message `0x4B2` to `0x695`.

For dialog id `0x6B`, `FUN_006040B0` maps:

| Hovered control | Status key pointer / meaning |
|---:|---|
| `0x6EB` | `STT:ScenarioListGameType` |
| `0x553` | `STT:ScenarioListMaps` |
| `0x468` | `STT:ScenarioMapThumbnail` |
| `0x6C5` | `STT:ScenarioButtonUseMap` |
| `0x583` | `STT:ScenarioButtonRandom` |
| `0x5C0` | `STT:ScenarioButtonCancel` |
| other / `0x695` itself | empty / null fallback |

Current Rust has `status_help_text` and renders it in the modal if non-empty, but this pass found no modal hover hit-test that sets those strings for `0x6B`.

Evidence: fresh decompile `FUN_00622B50`; fresh decompile context in `FUN_006040B0` showing the `iVar4 == 0x6b` branch.

### 3.5 Real listbox basics are mostly fixed, but wheel behavior is not verified

Active in YR: Yes for listboxes; Conditional for scrollbars when overflowed. `FUN_0060F9A0` maps `"ListBox"` to `OwnerDraw_ListBox_00618D40`, and `0x6B` controls `0x6EB`/`0x553` are real `LISTBOX` controls. Prior row-paint report verifies:

- row height is active font/text height + `2`, standard `19` px;
- selection fill spans the full content item rectangle;
- text starts at item-left `+2`;
- overflow creates a `20` px scrollbar and shrinks content width;
- custom hit-test uses `top_index + y / item_height` after client bounds checks.

Current Rust now matches the main geometry facts: `19` px rows, full content-row fill, `+2` text inset, and `20` px content shrink. The weak point is direct mouse wheel support: prior combo/listbox scrollbar report found no direct `WM_MOUSEWHEEL (0x20A)` case in scoped owner-draw listbox/scrollbar callbacks. Rust currently scrolls the modal list on wheel. This may be an ergonomic addition, but it is not binary-verified parity.

Evidence: fresh decompile `FUN_0060F9A0`; prior `SKIRMISH_OWNERDRAW_LISTBOX_00618D40_ROW_PAINT_GHIDRA_REPORT.md`; prior `SKIRMISH_0X102_COMBO_DROPDOWN_SCROLLBAR_GEOMETRY_GHIDRA_REPORT.md` negative wheel finding; current Rust scan `handle_choose_map_modal_mouse_wheel`.

### 3.6 Preview browsing boundary still matters

Active in YR: Yes. Prior callback slice proves passive `0x553` map-list highlight does not refresh preview. `WM_PAINT` draws the current `DAT_00AC1154`; Use Map commits via `0x005E7160`; parent return path refreshes selected preview after commit. Category `0x6EB` rebuilds `0x553` but still does not reload preview.

Current Rust aligns with this at state level: `select_map_filtered_row` only updates `highlighted_filtered_index`, and `commit_choose_map_selection` invalidates preview only on Use Map. However, preview drawing in the modal still uses the same committed preview texture path and needs screenshot validation against native `0x468` aspect fit/marker drawing.

Evidence: prior `SKIRMISH_CHOOSE_MAP_0X6B_PREVIEW_REFRESH_GHIDRA_REPORT.md`; fresh decompile `FUN_005E7160`.

### 3.7 Create Random Map remains a real command gap

Active in YR: Conditional on button click and selected mode allowing random maps. The `0x583` branch is a real callback branch and not a no-op. Existing RMG reports cover generated preview/RandMap lifecycle, including `RandMap.img` dimensions and 3-plane PCX-style decode risk.

Current Rust recognizes `0x583` but only logs. This is still a direct player-visible dead button.

Evidence: resource/control report; prior `SKIRMISH_CHOOSE_MAP_0X6B_PREVIEW_REFRESH_GHIDRA_REPORT.md` branch map; `GENERATETERRAINPREVIEW_RANDMAP_DIMENSIONS_COLORS_GHIDRA_REPORT.md`; current Rust scan.

## 4. INI Keys

No direct `rules.ini`, `rulesmd.ini`, `art.ini`, or `artmd.ini` keys control this modal's geometry, owner-draw row paint, hover/status mappings, or button behavior. `ini/mpmodesmd.ini` and scenario records provide row content/filtering, but not the modal visual/input contracts audited here.

YR `*md` precedence still matters for mode rows and random-map eligibility, but that is owned by the MPModes/scenario parser reports.

## 5. Integration Points

| Integration point | Verified behavior | Evidence | Current Rust note |
|---|---|---|---|
| Parent Choose Map action | parent opens modal `0x6B` and hides setup | `0x006ACEE0`, `0x005E68A0` prior/fresh evidence | now broadly represented |
| Modal asset binding | `MnScrnLCustomizeBattle` path for dialog `0x6B` | `0x0072D120`, `0x0060CF00` | now mostly represented for 800-wide shell |
| Common owner-draw setup | children are subclassed by class/style | `0x0060F9A0`, `0x00622820` | listboxes/buttons approximated, not full behavior |
| Status/help update | parent `WM_NCHITTEST` updates `0x695` | `0x00622B50`, `0x006040B0` | modal mappings missing |
| Accept | `0x005E7160` reads `0x553` then `0x6EB`, writes globals, closes result `1` | fresh decompile | Rust commits directly enough for current scope, load-failure/order still not exact |
| Cancel | callback closes result `2` and parent restores previous selection | prior callback/return reports | now no longer commits on Cancel |
| Random Map | `0x583` is real command path | resource/callback/RMG docs | still log-only |

## 6. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Modal wrapper `0x005E68A0` | verified | fresh decompile | none for ownership |
| Use Map helper `0x005E7160` | verified | fresh decompile | full parent load-failure restoration order is from sibling reports |
| Owner-draw class mapper `0x0060F9A0` | verified | fresh decompile | none for class/style mapping |
| Owner-draw button `0x00612B70` | touched-not-exhausted | fresh decompile | exact modal capture/release/default-button keyboard behavior needs focused slice |
| Owner-draw listbox `0x00618D40` | verified by prior exhaustive report, spot-confirmed via mapper | prior report + fresh `0x0060F9A0` | ordinary Win32 listbox default click/double-click specifics still deferred |
| Status parent handler `0x00622B50` | verified | fresh decompile | exact text rendering animation of `0x695` already covered by static reports |
| Tooltip mapper `0x006040B0`, `0x6B` branch | verified | fresh decompile context | none for mapped controls listed here |
| Current Rust layout/state/render scan | verified | source scan after current implementation | implementation |
| Runtime screenshot parity | deferred | none | capture native vs Rust 800x600 and 1024x768 |
| Create Random Map generator | deferred | existing RMG docs | implement/audit as separate system |

## 7. Open Questions - Final State

- `[RESOLVED] OQ-01 - Does current Rust still draw parent setup behind the chooser? -> Mostly no; modal instance building returns early when `choose_map_layout` exists.` (evidence: current `src/app_skirmish_shell_render.rs` scan)
- `[RESOLVED] OQ-02 - Is `MnScrnLCustomizeBattle` still missing from current Rust? -> No; current Rust has a modal background atlas entry and classifier.` (evidence: current `src/render/skirmish_shell_chrome.rs` scan)
- `[RESOLVED] OQ-03 - Are title/preview/status static rects still wrong? -> Mostly no; current Rust now uses resource rects for `0x694`, `0x468`, `0x695`, and headings.` (evidence: current `compute_choose_map_modal_layout`)
- `[RESOLVED] OQ-04 - Are modal button rects still wrong? -> Yes; current Rust uses old upper-right 112x30 button rects instead of resource 108x23 rects with bottom-right Cancel.` (evidence: resource report; current layout scan)
- `[RESOLVED] OQ-05 - Does `0x6B` have status/help mappings? -> Yes, `FUN_006040B0` maps controls `0x6EB`, `0x553`, `0x468`, `0x6C5`, `0x583`, and `0x5C0` to `STT:Scenario*` keys.` (evidence: fresh `FUN_006040B0` decompile context)
- `[RESOLVED] OQ-06 - Does current Rust populate `0x6B` status/help text from hover? -> No modal hover mapping was found; only generic status text state/render exists.` (evidence: current Rust scan)
- `[RESOLVED] OQ-07 - Does current Rust model modal button pressed state? -> No, modal `push_button_30` calls pass `pressed=false` and app activates on mouse down.` (evidence: current app/render scan)
- `[RESOLVED] OQ-08 - Does owner-draw button code make pressed state visible/audio-relevant? -> Yes, `OwnerDraw_Button_00612B70` uses down/disabled state and paint-transition sound behavior.` (evidence: fresh decompile)
- `[RESOLVED] OQ-09 - Is wheel scrolling verified for real owner-draw listboxes? -> Not in scoped callbacks; prior reports found no direct `0x20A` case.` (evidence: prior scrollbar/listbox report)
- `[RESOLVED] OQ-10 - Is Create Random Map still missing? -> Yes, current Rust logs only.` (evidence: current `src/app.rs` scan)
- `[DEFERRED] OQ-11 - Exact `LAB_005E6920` keyboard/default/double-click behavior.` (category: `requires-different-system-context`; reason: callback still lacks a clean function boundary and this pass avoided Ghidra mutation; next-step-if-pursued: run a read-only disassembly-focused input report or approve boundary creation)
- `[DEFERRED] OQ-12 - Runtime RGB/pixel comparison for modal first paint.` (category: `needs-runtime-debugger`; reason: no native screenshot capture in this pass; next-step-if-pursued: capture native/Rust at 800x600 and 1024x768)
- `[DEFERRED] OQ-13 - Full random-map command implementation contract.` (category: `out-of-scope`; reason: existing RMG reports must be consumed as a separate implementation contract; next-step-if-pursued: synthesize RMG -> Choose Map handoff)

## 8. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Modal resource buttons are Use Map `(425,122,108,23)`, Create Random `(425,149,108,23)`, Cancel `(425,346,108,23)` in local `0x6B` coords. | resource report; command ids in callback reports | mismatch | `src/ui/skirmish_shell/layout.rs`, `src/app.rs`, `src/app_skirmish_shell_render.rs` | Move hit/render rects to resource positions and sizes; Cancel must be bottom-right. | At 800x600, clicking old upper Cancel area does nothing; clicking resource bottom-right Cancel closes unchanged. | Do not preserve current upper-right Cancel placement because it is visually obvious drift. |
| Modal push buttons are owner-drawn buttons with down/disabled paint and sound gates. | `FUN_0060F9A0`; `OwnerDraw_Button_00612B70`; push-button sound report | partial: modal buttons are always rendered unpressed and activated on mouse down | app input state + render button state | Track pressed modal button, render down state while captured, activate on release-over-same control, clear on drag-off/release. | Mouse down on Use Map shows pressed art; drag off and release does not commit; release over same commits. | Do not model modal buttons as immediate fire-on-mousedown commands. |
| Dialog `0x6B` status strip maps hovered controls to `STT:Scenario*` help strings. | `FUN_00622B50`; `FUN_006040B0` `iVar4 == 0x6B` branch | missing | `src/app.rs`, `src/ui/skirmish_shell/state.rs`, `src/app_skirmish_shell_render.rs` | Add modal hover hit testing that sets/clears `status_help_text` for `0x6EB`, `0x553`, `0x468`, `0x6C5`, `0x583`, `0x5C0`. | Hover map list shows scenario-list-map help; hover blank dialog/status strip clears help. | Do not render permanent `GUI:Blank` or hardcoded English status text. |
| Real listbox wheel handling is not verified in scoped native callbacks. | prior listbox/scrollbar report negative for direct `0x20A` | Rust currently scrolls on wheel | `src/app.rs::handle_choose_map_modal_mouse_wheel` | Decide whether to remove, gate as non-parity convenience, or verify parent translation in a focused callback pass. | With parity mode, mouse wheel over Choose Map list either matches native verified behavior or is consciously documented as drift. | Do not silently keep convenient wheel behavior and call it native. |
| Passive map-row highlight does not refresh preview; Use Map is commit boundary. | `SKIRMISH_CHOOSE_MAP_0X6B_PREVIEW_REFRESH_GHIDRA_REPORT.md`; `0x005E7160` | mostly matching | `ChooseMapModalState`, app commit path, preview cache | Preserve separation; add regression coverage around Cancel/highlight/Use Map. | Highlight a different map then Cancel: parent preview unchanged; Use Map invalidates committed preview after close. | Do not make the modal preview "live update" on row highlight. |
| `0x583` Create Random Map is a real command path. | resource report; callback branch; RMG reports | missing/log-only | app modal action path, random map generator/preview decode surfaces | Wire to verified RMG flow or expose a disabled/blocked state until RMG is implemented. | Clicking Create Random Map does not silently do nothing in player-visible UI. | Do not leave player-facing log-only behavior. |
| `MnScrnLCustomizeBattle.shp` is loaded only for exact `g_ScreenWidth == 800`; larger widths need runtime validation. | `0x0072D120`; visual layout report | partial: Rust draws asset only when layout screen width is 800 | chrome/render resolution policy | Keep exact-800 behavior unless runtime screenshot proves fallback/stretch behavior for larger screens. | 1024x768 modal screenshot matches native fallback/centering, not an assumed stretch. | Do not stretch 800 art across wider shell without evidence. |

## 9. Highest-Leverage Follow-Up Queue

1. **Fix modal button geometry and pressed/capture behavior.** This is obvious every time the dialog opens and has clear binary evidence.
2. **Implement `0x6B` status/help hover mappings.** This is verified, local to UI/app state, and gives the modal more native feel without touching sim.
3. **Decide wheel behavior.** Either verify a parent translation path or remove/gate the current unverified wheel scroll.
4. **Create Random Map contract/implementation.** Larger than a UI polish task because it touches `RandMap.Sed`, `RandMap.img`, generated preview decode, and RMG state.
5. **Runtime screenshot pass.** Needed before calling visuals close: 800x600 and 1024x768 native vs Rust, with buttons/listboxes/status/preview checked.

## 10. Stale Docs / Follow-Up Docs

- `SKIRMISH_CHOOSE_MAP_0X6B_CURRENT_MODAL_RECHECK_GHIDRA_REPORT.md`: its Rust-status rows saying parent setup still draws under the modal, `MnScrnLCustomizeBattle` is missing, title/preview geometry is wrong, and Cancel commits saved selection are stale after the current implementation. Replace with the current delta snapshot in this report.
- `SKIRMISH_OWNERDRAW_LISTBOX_00618D40_ROW_PAINT_GHIDRA_REPORT.md`: its Rust-status row saying `CHOOSE_MAP_LIST_ROW_H` is `16` is stale. Current Rust uses `19` and has shared content-shrink helpers.
- Prior open question "Do non-0x102 shell dialogs have different `0x695` update semantics?" is resolved for `0x6B`: `FUN_006040B0` has an explicit `0x6B` branch mapping scenario-list/button/thumbnail controls.

## Sources

- Fresh Ghidra read-only decompile: `FUN_005e68a0`, `FUN_005e7160`, `FUN_0060f9a0`, `OwnerDraw_Button_00612B70`, `FUN_00622b50`.
- Fresh Ghidra read-only tooltip branch evidence: `FUN_006040B0` decompile context, dialog id `0x6B` cases for `0x6EB`, `0x553`, `0x468`, `0x6C5`, `0x583`, `0x5C0`.
- Existing reports referenced: `SKIRMISH_CHOOSE_MAP_MODAL_VISUAL_CONTROL_LAYOUT_GHIDRA_REPORT.md`, `SKIRMISH_OWNERDRAW_LISTBOX_00618D40_ROW_PAINT_GHIDRA_REPORT.md`, `SKIRMISH_CHOOSE_MAP_0X6B_PREVIEW_REFRESH_GHIDRA_REPORT.md`, `SKIRMISH_CHOOSE_MAP_0X6B_CURRENT_MODAL_RECHECK_GHIDRA_REPORT.md`, `SKIRMISH_OWNERDRAW_PUSH_BUTTON_SOUNDS_GHIDRA_REPORT.md`, `GENERATETERRAINPREVIEW_RANDMAP_DIMENSIONS_COLORS_GHIDRA_REPORT.md`.
- Rust scanned read-only: `src/app.rs`, `src/ui/skirmish_shell/layout.rs`, `src/ui/skirmish_shell/state.rs`, `src/ui/skirmish_shell/mod.rs`, `src/app_skirmish_shell_render.rs`, `src/render/skirmish_shell_chrome.rs`.
- INI scan: `ini/rules.ini`, `ini/rulesmd.ini`, `ini/art.ini`, `ini/artmd.ini`, `ini/mpmodesmd.ini`.
