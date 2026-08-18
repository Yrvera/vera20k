# Skirmish Choose Map Modal First Paint Visual Trace

**Scenario:** Standard offline YR Skirmish setup at 800x600, click Choose Map, trace first paint of dialog `0x6B`.

**Scope:** Modal background/chrome, mode listbox `0x6EB`, map listbox `0x553`, Use Map/Cancel/Create Random Map buttons, preview/static area, static text, list row paint, and current Rust output.

**Verdict Tally:** PASS: 1 | FAIL: 5 | UNCHECKED: 2 | NOT-IMPLEMENTED: 8

## Current Rust Status Correction - 2026-05-23

The current-Rust pipeline and several FAIL/NOT-IMPLEMENTED rows below are
superseded by
`skirmish-ui/SKIRMISH_CHOOSE_MAP_0X6B_CURRENT_MODAL_RECHECK_GHIDRA_REPORT.md`.
Current Rust now reaches
`ChooseMap0x5aa -> SkirmishShellAction::ChooseMap -> open_choose_map_modal -> ChooseMapModalState`,
then renders a primitive modal overlay and text/list rows. The current
first-paint mismatch is no longer absence of state/rendering; it is that the
parent setup remains drawn underneath, modal background uses primitive solid
rectangles instead of `MnScrnLCustomizeBattle` assets, and several `0x6B`
resource rects/statics/buttons remain wrong or absent. The button-geometry
mismatch remains current: gamemd resource buttons are Use Map
`(425,122,108,23)`, Create Random Map `(425,149,108,23)`, and Cancel
`(425,346,108,23)`, while current Rust still uses non-resource modal button
rects in `compute_choose_map_modal_layout`.

## Pipeline

`0x102 Choose Map button 0x5AA click -> parent hides -> modal wrapper 0x005E68A0 creates resource 0x6B -> shell setup loads MnScrnLCustomizeBattle.* -> 0x6EB/0x553 owner-drawn listboxes populate -> owner-drawn buttons/statics/preview control paint -> modal first frame visible`

Current Rust now reaches:

`ChooseMap0x5aa hit-test -> SkirmishShellAction::ChooseMap -> open_choose_map_modal -> ChooseMapModalState -> primitive modal overlay/list/text rows over the setup shell`

## Findings

### Stage 1 - Trigger to modal screen

- **gamemd:** `0x006ACEE0` hides the setup HWND, calls `0x005E68A0`, creates dialog resource `0x6B`, shows chooser, and pumps the modal. Active in standard offline YR per `SKIRMISH_CHOOSE_MAP_MODAL_FLOW_GHIDRA_REPORT.md`.
- **Rust:** click identifies `ChooseMap0x5aa`, but `handle_skirmish_shell_action` treats `ChooseMap` as no-op at `src/app.rs:585`.
- **Player-visible result:** clicking Choose Map leaves the setup screen visible; no modal first paint occurs.
- **Verdict:** FAIL.

### Stage 2 - App-owned modal state

- **gamemd:** parent saves old selected mode/map, hides setup, and owns a separate modal HWND until return.
- **Rust:** `ChooseMapModalState` is now active through the app path; `open_choose_map_modal` stores modal state and `handle_choose_map_modal_mouse_down` handles Use Map, Cancel, and list clicks.
- **Player-visible result:** there is a modal lifecycle, but it is a primitive overlay over the setup shell rather than gamemd's separate hidden-parent `0x6B` dialog.
- **Verdict:** SUPERSEDED: PARTIAL.

### Stage 3 - Dialog `0x6B` background/chrome asset

- **gamemd:** dialog `0x6B` uses `MnScrnLCustomizeBattle.shp` at 800 width and `MnScrnLCustomizeBattle.PAL`; active in standard YR per `SKIRMISH_CHOOSE_MAP_MODAL_VISUAL_CONTROL_LAYOUT_GHIDRA_REPORT.md`.
- **Rust:** the shell atlas loads `MNSCRNS.SHP` and `MnScrnLCoopGameSetup.shp`, not `MnScrnLCustomizeBattle.shp`, at `src/render/skirmish_shell_chrome.rs:166`; `MnScrnLCustomizeBattle.shp` remains classified as a research candidate at `src/render/skirmish_shell_chrome.rs:357`.
- **Player-visible result:** even if the modal were entered, it would not have the retail Choose Map background/chrome.
- **Verdict:** NOT-IMPLEMENTED.

### Stage 4 - Dialog and listbox geometry model

- **gamemd:** resource `0x6B` is `533x369`; at 800x600 centered in the shell base this gives dialog `(133,115,533,369)`. Verified controls: `0x6EB` local `(77,78,130,211)` -> screen `(210,193,130,211)`, `0x553` local `(225,78,130,211)` -> screen `(358,193,130,211)`.
- **Rust:** `compute_choose_map_modal_layout(800,600)` returns those same dialog/list rectangles at `src/ui/skirmish_shell/layout.rs:508`, with test assertions at `src/ui/skirmish_shell/layout.rs:743`.
- **Player-visible result:** the in-memory helper has the verified modal/listbox bounds, but this helper is not consumed by rendering yet.
- **Verdict:** PASS for helper geometry only.

### Stage 5 - Button geometry

- **gamemd:** resource `0x6B` buttons are local `UseMap 0x6C5=(425,122,108,23)`, `CreateRandomMap 0x583=(425,149,108,23)`, `Cancel 0x5C0=(425,346,108,23)`.
- **Rust:** helper uses local `UseMap=(374,80,112,30)`, `Cancel=(374,116,112,30)`, `CreateRandomMap=(374,152,112,30)` at `src/ui/skirmish_shell/layout.rs:518`.
- **Player-visible result:** if drawn, buttons would be horizontally/vertically misplaced and wrong-sized; Cancel would appear in the upper right instead of the bottom right.
- **Verdict:** FAIL.

### Stage 6 - Title/static/preview geometry

- **gamemd:** resource `0x6B` has title static `0x694=(425,1,108,10)`, preview/static `0x468=(428,23,96,69)`, heading statics `(77,60,130,10)` and `(225,60,130,10)`, select-engagement static `(80,20,257,12)`, status strip `0x695=(2,355,303,12)`.
- **Rust:** `ChooseMapModalLayout` only models `title` and `preview` at `src/ui/skirmish_shell/layout.rs:176`; their current rects are local title `(0,20,533,24)` and preview `(374,202,128,96)` at `src/ui/skirmish_shell/layout.rs:521`.
- **Player-visible result:** the title/preview would be in the wrong locations and several required static labels/status text areas have no represented rectangle.
- **Verdict:** FAIL.

### Stage 7 - Modal rendering path

- **gamemd:** first paint draws the separate dialog `0x6B` after setup is hidden.
- **Rust:** `render_skirmish_shell_with_atlas` always computes and draws setup `0x102` through `compute_layout`, `build_skirmish_shell_instances`, and `build_shell_text_draws` at `src/app_skirmish_shell_render.rs:1818`; no branch calls `compute_choose_map_modal_layout`.
- **Player-visible result:** there are no modal sprites, no modal text, and no modal scissor/list areas.
- **Verdict:** NOT-IMPLEMENTED.

### Stage 8 - Owner-drawn modal buttons

- **gamemd:** Use Map, Cancel, and Create Random Map are owner-drawn shell buttons using the normal 30-height button PCX path (`bue_*30` / `bde_*30`).
- **Rust:** reusable `push_button_30` exists at `src/app_skirmish_shell_render.rs:343`, but it is only called for the setup Start/Back/Choose Map buttons at `src/app_skirmish_shell_render.rs:1171`.
- **Player-visible result:** modal buttons do not render.
- **Verdict:** NOT-IMPLEMENTED.

### Stage 9 - Listbox frame, rows, scrollbars

- **gamemd:** `0x6EB` and `0x553` are real `LISTBOX` controls with owner-draw fixed style `0x151`; row backing uses item data, no display-name sort, and scrollbar PCXs when needed.
- **Rust:** there is no modal listbox renderer. Existing dropdown rendering uses generic solid fills/outlines and combo scrollbar helpers at `src/app_skirmish_shell_render.rs:640`, not dialog `0x6B` listboxes.
- **Player-visible result:** no listbox frame, no highlighted row, no native scrollbar, and no mode/map rows appear.
- **Verdict:** NOT-IMPLEMENTED.

### Stage 10 - List row height and text insets

- **gamemd:** exact owner-drawn listbox row internals were explicitly left as remaining uncertainty in the visual-layout report.
- **Rust:** helper hit testing assumes `CHOOSE_MAP_LIST_ROW_H = 16` at `src/ui/skirmish_shell/layout.rs:30`, but no retail row-paint numbers were computed for comparison.
- **Player-visible result:** row cadence/text inset parity cannot be claimed.
- **Verdict:** UNCHECKED.

### Stage 11 - Mode/map row content source

- **gamemd:** mode list `0x6EB` displays MPModes rows; map list `0x553` displays source-order scenario records filtered by selected mode.
- **Rust:** data helpers exist (`ChooseMapModalState` at `state.rs:102`, scenario filtering in `skirmish_scenarios.rs`), but no app integration feeds these rows into a rendered modal.
- **Player-visible result:** first paint cannot show Battle/Team/etc. rows or map names.
- **Verdict:** NOT-IMPLEMENTED.

### Stage 12 - Preview/static control behavior

- **gamemd:** control `0x468` exists at local `(428,23,96,69)`; the visual report verifies the control, but live chooser-specific preview paint was not fully drained.
- **Rust:** no modal preview/static renderer exists, and current helper preview rect is wrong.
- **Player-visible result:** modal preview area is absent; exact native live-preview behavior remains unresolved.
- **Verdict:** UNCHECKED for native live-preview behavior; NOT-IMPLEMENTED for Rust modal preview paint.

### Stage 13 - Text rendering for modal labels

- **gamemd:** labels include `GUI:ChooseMap`, `GUI:SelectEngagement`, `GUI:GameType`, `GUI:GameMap`, `GUI:Blank`, and button labels from the resource.
- **Rust:** `build_shell_text_draws` renders setup labels only; modal labels are absent at `src/app_skirmish_shell_render.rs:1423`.
- **Player-visible result:** no modal captions, headings, button text, or bottom status/help text.
- **Verdict:** NOT-IMPLEMENTED.

### Stage 14 - Parent hide/show visual ordering

- **gamemd:** parent setup hides before the chooser appears and is shown again only after chooser return.
- **Rust:** parent setup remains the only rendered shell surface after Choose Map because the action is swallowed.
- **Player-visible result:** the screen never transitions into a separate modal surface; this is the largest visible mismatch.
- **Verdict:** FAIL.

### Stage 15 - First-paint draw order

- **gamemd:** modal background/chrome must appear behind owner-drawn listboxes/buttons/statics; setup controls are not visible behind it.
- **Rust:** no modal draw-order function exists; setup draw order remains active through `skirmish_shell_semantic_draw_order` at `src/app_skirmish_shell_render.rs:1067`.
- **Player-visible result:** no modal layering can be validated or matched.
- **Verdict:** NOT-IMPLEMENTED.

### Stage 16 - Retail asset coverage for first paint

- **gamemd:** first paint needs `MnScrnLCustomizeBattle.shp/.PAL`, owner-drawn button PCXs, listbox primitive/scrollbar art, and shell text.
- **Rust:** button PCXs and scrollbar PCXs are loaded in the atlas at `src/render/skirmish_shell_chrome.rs:188`, but the modal background pair is absent and no modal-specific draw path consumes any of them.
- **Player-visible result:** partial asset coverage exists in memory, but the modal still paints nothing.
- **Verdict:** FAIL.

## Top Player-Visible Gaps

1. Clicking Choose Map now opens Rust modal state, but the parent setup remains drawn underneath; gamemd `0x006ACEE0` hides setup and calls modal wrapper `0x005E68A0`.
2. Modal background/chrome is missing; atlas loads setup backgrounds at `src/render/skirmish_shell_chrome.rs:166`; gamemd `0x6B` uses `MnScrnLCustomizeBattle.shp/.PAL`.
3. Modal buttons are modeled at wrong rects; Rust uses local `(374,80,112,30)` etc. at `src/ui/skirmish_shell/layout.rs:518`; gamemd resource uses `UseMap=(425,122,108,23)`, `CreateRandomMap=(425,149,108,23)`, `Cancel=(425,346,108,23)`.
4. Modal title/preview/statics are wrong or absent; Rust title/preview at `layout.rs:521` do not match gamemd `0x694=(425,1,108,10)`, `0x468=(428,23,96,69)`, headings/status statics.
5. Listboxes now render through a primitive modal path, but not through gamemd's owner-drawn `0x6EB` and `0x553` listbox paint path.

## Adjacent Findings

- The setup-shell renderer is more mature than the chooser modal renderer: right panel, setup buttons, combo faces, flags, checkboxes, trackbars, and preview texture paths exist, but they target dialog `0x102`, not modal `0x6B`.
- Existing research docs still contain older Rust contrast notes saying Choose Map cycles selected maps; current Rust has been improved to bubble `ChooseMap`, but app routing still makes it a visible no-op.

## Recommended Next Fix Order

1. Add app-owned active choose-map modal state and route `SkirmishShellAction::ChooseMap` into it.
2. Fix `ChooseMapModalLayout` to model all verified `0x6B` controls, especially buttons, title, preview, headings, select-engagement text, and status strip.
3. Extend the shell chrome atlas with `MnScrnLCustomizeBattle.shp` and `MnScrnLCustomizeBattle.PAL`.
4. Add a modal render branch that draws `0x6B` background, statics, buttons, and two listboxes while suppressing setup controls behind it.
5. Implement owner-drawn listbox row paint/scrollbars, then re-run a focused row-cadence trace once exact native row insets are verified.

## Sources

- `docs/research/skirmish-ui/SKIRMISH_CHOOSE_MAP_MODAL_VISUAL_CONTROL_LAYOUT_GHIDRA_REPORT.md`
- `docs/research/skirmish-ui/SKIRMISH_CHOOSE_MAP_MODE_CATEGORY_0X6EB_GHIDRA_REPORT.md`
- `docs/research/skirmish-ui/SKIRMISH_CHOOSE_MAP_ACCEPT_CANCEL_SIDE_EFFECTS_GHIDRA_REPORT.md`
- `docs/research/skirmish-ui/SKIRMISH_CHOOSE_MAP_PREVIEW_REFRESH_FUN_006ACEE0_GHIDRA_REPORT.md`
- `docs/research/skirmish-ui/SKIRMISH_CHOOSE_MAP_MODAL_FLOW_GHIDRA_REPORT.md`
- Rust contrast scan: `src/ui/skirmish_shell/layout.rs`, `src/ui/skirmish_shell/state.rs`, `src/app.rs`, `src/app_skirmish_shell_render.rs`, `src/render/skirmish_shell_chrome.rs`.
