# Skirmish Choose Map Modal 0x6B Visual Integration - Ghidra Research Report

**Address(es):** `0x005E68A0`, callback entry `0x005E6920`, `0x0060CF00`, `0x00622820`, `0x0060C540`, `0x0072D120`, `0x0072D170`, `0x0072D210`, `0x005E7160`, `0x006ACEE0`  
**Investigation Mode:** exhaustive-slice scoped to visual integration blockers and handoff, extending existing Choose Map reports rather than re-covering modal flow.  
**Target question:** What remains needed to visually integrate the gamemd-alike Choose Map modal dialog `0x6B` into the Skirmish shell?  
**Non-goals:** map preview decode internals, base setup `0x102` parent background except overlay boundary, random-map generator internals, and Rust/INI/doc edits outside this report.  
**Evidence needed to mark COMPLETE:** prior docs reconciled, current Rust surfaces scanned, Ghidra spot-checks for modal asset/layout/buttons/overlay path, all material claims marked Active in YR with evidence, and remaining uncertainty explicitly listed.  
**Stop conditions:** write exactly this report, keep Ghidra read-only, no Rust changes, no untriaged open questions.  
**Claimed Scope:** `MnScrnLCustomizeBattle.shp/.PAL` modal background path, modal shell lifecycle, listbox/control layout contract, Use Map/Cancel/Create Random Map button visuals, preview control placement, parent hide/show/invalidation boundary, and current Rust delta.  
**Non-Scope:** exact `PreviewPack` decode, exact random-map generator UI after pressing `0x583`, and byte-perfect `OwnerDraw_ListBox_00618D40` row paint internals beyond verified owner-draw classification and resource/list message contract.  
**Confidence:** High for active modal lifecycle, `0x6B` asset/palette selection, shell fullscreen setup, resource geometry, button command/result wiring, parent hide/show, and Rust integration gap. Medium for selected list row styling because `OwnerDraw_ListBox_00618D40` times out in Ghidra and remains a focused follow-up.  
**Active in YR:** Yes for the standard offline Skirmish Choose Map path; conditional only for exact branches tied to selected button/result or screen width.

## 1. Overview

Retail Choose Map is a separate shell dialog (`RT_DIALOG 0x6B`) created from the offline Skirmish `0x102` Choose Map command. The parent setup dialog is hidden before the modal is shown, then restored after modal return; the chooser is not painted as a child overlay while setup controls remain live.

The modal visual blocker in Rust is not lack of state: Rust already has `ChooseMapModalState` and geometry tests. The blocker is integration: the render path has no modal draw state, no packed `MnScrnLCustomizeBattle.*` background entry, no listbox/button/preview renderer for `0x6B`, and the `ChooseMap` action is currently returned but swallowed by `app.rs`.

## 2. Verified Binary Findings

| Finding | Active in YR | Evidence | Implementation meaning |
|---|---|---|---|
| Choose Map wrapper creates dialog id `0x6B` with callback `0x005E6920`, stores HWND in `DAT_00AC0D40`, sends `0x4A9`, shows it, then pumps modal loop. | Yes | `0x005E68B7 MOV EDX,0x6B`; `0x005E68BE PUSH 0x005E6920`; `0x005E68C4 CALL 0x00775700`; `0x005E68D0`; `0x005E68E3`; `0x005E68F5`; `0x005E690F` | Rust needs a modal/screen state for `0x6B`, not an in-place selected-map change. |
| `MnScrnLCustomizeBattle.shp/.PAL` belongs to dialog `0x6B` visual setup, not base setup `0x102`. | Yes for dialog `0x6B`; SHP load conditional on `g_ScreenWidth == 800`. | Strings at `0x00844FE0` / `0x00844FC4`; `0x0072D129 CMP g_ScreenWidth,0x320`; `0x0072D135` loads pointer table `0x00844D64`; `0x0072D14A..0x0072D15A` loads PAL through `0x0072ADE0`; `0x0060CF00` branch `iVar3 == 0x6B` writes palette `FUN_0072D210()` and SHP `DAT_00B0FAB8`. | Promote these assets from candidate to modal-specific render assets, but do not use them for base setup `0x102`. |
| `0x6B` participates in common fullscreen shell setup. | Yes | `0x00622820` calls owner-draw setup, writes dialog id, then `0x0060C540`; `0x0060C540` includes `0x6B` in the fullscreen-shell set; `0x00622820` fullscreen branch calls `MoveWindow(parent,0,0,g_ScreenWidth,g_ScreenHeight,0)` and enumerates `ResizeShellChildControl_0060C0C0`. | Center modal controls inside the shell coordinate system after fullscreen parent move; do not scale the dialog controls. |
| `0x6B` resource layout has two owner-drawn listboxes (`0x6EB`, `0x553`), three right-column buttons (`0x6C5`, `0x583`, `0x5C0`), title static `0x694`, status/help static `0x695`, and preview static `0x468`. | Yes | Prior resource extraction in `SKIRMISH_CHOOSE_MAP_MODAL_VISUAL_CONTROL_LAYOUT_GHIDRA_REPORT.md`: DIALOGEX `0x6B`, rect `(0,0,533,369)`, 11 controls; Ghidra spot checks `0x005E6EA6` and `0x005E6F17`; accept helper reads `0x553`/`0x6EB` via listbox messages. | Use the `0x6B` control inventory; do not copy setup `0x102` labels/rows/checkboxes into the chooser. |
| Buttons are normal shell owner-draw push buttons. | Yes for `0x6C5` and `0x5C0`; conditional for `0x583` when player chooses random map. | Resource styles `0x5000000B`; owner-draw setup maps Button with `(style & 0xB)==0xB` to `OwnerDraw_Button_00612B70` at `0x0060FE58..0x0060FEFE`; command dispatch checks `0x6C5`, `0x583`, and `0x5C0` at `0x005E69C2..0x005E69EC`, with Use Map calling `0x005E7160` at `0x005E6B63`. | Render Use Map/Cancel/Create Random Map with the same `bue_*30`/`bde_*30` shell button pieces and pressed offsets as other shell buttons. |
| Listboxes are real owner-drawn `LISTBOX` controls, not combos. | Yes | Resource class/style `LISTBOX 0x50000151`; owner-draw setup maps `"ListBox"` to `OwnerDraw_ListBox_00618D40` at `0x0060FC18..0x0060FC29`; accept uses `LB_GETCURSEL 0x188` and `LB_GETITEMDATA 0x199` on `0x553` and `0x6EB`. | Render two fixed owner-draw list surfaces with item-data-backed selection; do not reuse combo dropdown geometry directly. |
| Parent setup is hidden while chooser is active and restored afterward; repaint/preview work happens after return. | Yes | Parent `0x006ACEE0` branch for command `0x5AA` calls `ShowWindow(setup,0)` before `0x005E68A0`; accepted/cancel branches call `ShowWindow(setup,5)` and preview invalidation/refresh paths; `InvalidateRect(..., FALSE)` appears on preview refresh branches. | Modal must block parent setup hit-testing and visual composition until it returns; accepted/cancel return controls setup update timing. |

## 3. Current Rust Status

| Rust surface | Status | Evidence |
|---|---|---|
| Modal state/filter model | Exists | `src/ui/skirmish_shell/state.rs` defines `ChooseMapModalState`, saved selection, filtered records, accept/cancel/random-map methods and tests. |
| Modal layout/hit tests | Partial | `src/ui/skirmish_shell/layout.rs` defines `compute_choose_map_modal_layout`, buttons, list hit tests, and tests. Geometry is close to the verified resource but uses hand-entered pixel rects and omits static labels/status rects in the public layout struct. |
| App action routing | Missing integration | `src/app.rs` calls `apply_action`, then swallows `SkirmishShellAction::ChooseMap` in the no-op arm. |
| Modal render path | Missing | `rg` found no `choose_map_modal` rendering in `src/app_skirmish_shell_render.rs`; the render path draws only base setup shell, preview, markers, and text. |
| Modal assets | Missing | `src/render/skirmish_shell_chrome.rs` keeps `MnScrnLCustomizeBattle.shp` classified as `ResearchCandidate` in tests and does not pack `MnScrnLCustomizeBattle.PAL` or expose a modal background entry. |
| Listbox row styling | Partial/unchecked | Rust has list hit-test row height constant `16`, but no visible modal listbox renderer or selected-row skin contract. |

## 4. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| `0x6B` is a separate modal shell dialog shown after hiding setup. | `0x006ACEE0` `ShowWindow(setup,0)` then `0x005E68A0`; `0x005E68F5` show chooser | missing | `src/app.rs`, `src/ui/skirmish_shell/state.rs`, `src/app_skirmish_shell_render.rs` | Add explicit active chooser state, route `ChooseMap` to open it, block parent hit-testing, and draw modal instead of setup controls while active. | Click `Choose Map`: setup controls disappear/stop responding; chooser opens centered; Cancel returns without changing selected map. | Do not render as a transparent child panel over still-interactive setup controls. |
| `MnScrnLCustomizeBattle.shp/.PAL` is the verified `0x6B` background/palette path, SHP at exactly 800-wide load. | `0x0072D120`, `0x0060CF00`, strings `0x00844FE0`/`0x00844FC4` | missing; currently candidate-only | `src/render/skirmish_shell_chrome.rs`, render atlas structs | Pack modal background with its PAL and expose a modal-specific atlas entry; draw it for 800-wide modal composition and use verified fallback policy elsewhere. | 800x600 chooser background matches retail `Customize Battle` shell art instead of primitive/blank overlay. | Do not use `MnScrnLCoopGameSetup.*` for the chooser; do not use `MnScrnLCustomizeBattle.*` for base `0x102`. |
| Buttons `0x6C5`, `0x5C0`, `0x583` are owner-draw push buttons with shell button art. | resource style `0x5000000B`; `0x0060FE58`; command dispatch `0x005E69C2..0x005E69EC` | partly available button art, not modal-connected | `src/app_skirmish_shell_render.rs`, `src/ui/skirmish_shell/layout.rs` | Draw Use Map/Cancel/Create Random Map via existing shell button pieces and wire pressed state/action return. | Press/release Use Map commits highlighted map; Cancel restores; buttons visually depress with `bde_*30`. | Do not invent separate Choose Map button art. |
| Listboxes `0x6EB`/`0x553` are owner-drawn listboxes with item-data selection. | resource style `0x50000151`; `0x0060FC18`; `0x005E7160` messages `0x188`/`0x199` | state exists, render absent | `src/app_skirmish_shell_render.rs`, `src/ui/skirmish_shell/state.rs` | Render two fixed listboxes with selected row styling and scroll/top-index state; preserve source/filter order and stable record ids. | Selecting a mode updates map rows; selecting a map highlights row; Use Map commits exact record id. | Do not sort by display name; do not model `0x6EB` as a combo. |
| Preview control `0x468` is part of the chooser layout, but normal row browsing does not replace the preview. | resource `0x468`; `SKIRMISH_CHOOSE_MAP_0X6B_PREVIEW_REFRESH_GHIDRA_REPORT.md` verifies `0x005E6920` paints current `DAT_00AC1154`, while passive `0x553` highlight and `0x6EB` category rebuild do not call preview loaders | missing/unchecked modal render | `src/app_skirmish_shell_render.rs`, preview texture state | Draw the preview frame/placeholder from the current committed preview while the modal is open; refresh preview only after Use Map commits and the parent `0x102` return path reloads state. | Highlight rows in Choose Map: preview remains old/current; Use Map commits and then refreshes the setup preview. | Do not make the chooser preview "live" on row highlight. |

### Proposed Rust Test Names

- `choose_map_action_opens_modal_and_blocks_parent_shell_controls`
- `choose_map_modal_uses_mnscrnlcustomizebattle_background_at_800`
- `choose_map_modal_buttons_use_shell_ownerdraw_button_skin`
- `choose_map_modal_listboxes_preserve_item_data_and_source_order`
- `choose_map_modal_cancel_restores_saved_selection_without_preview_commit`

## 5. Negative Facts / Do Not Do

- Do not treat `MnScrnLCustomizeBattle.shp` as unverified generally. Active in YR: Yes for chooser `0x6B`; evidence `0x0072D120` + `0x0060CF00`. The correct distinction is "verified modal asset, not base setup asset."
- Do not render Choose Map as in-place map cycling. Active in YR: No for standard Choose Map; evidence parent `0x006ACEE0` hides setup and calls modal wrapper `0x005E68A0`.
- Do not copy base setup `0x102` controls into the chooser. Active in YR: No; evidence resource `0x6B` has 11 controls and lacks `0x6EC`/`0x5A8` setup labels, player rows, checkboxes, and trackbars.
- Do not use combo dropdown behavior for `0x6EB` or `0x553`. Active in YR: No; evidence both are `LISTBOX` controls and accept uses `LB_GETCURSEL`/`LB_GETITEMDATA`.
- Do not leave parent setup interactive under the chooser. Active in YR: No; evidence `ShowWindow(setup,0)` before modal creation and `ShowWindow(setup,5)` after return.

## 6. Remaining Uncertainty

- Exact selected row pixel styling for `OwnerDraw_ListBox_00618D40` is resolved for basic Choose Map listbox rows by `SKIRMISH_OWNERDRAW_LISTBOX_00618D40_ROW_PAINT_GHIDRA_REPORT.md`: font+2 row height, full-row selection fill, `+2` text inset, and 20 px scrollbar content shrink.
- Live preview update while browsing inside dialog `0x6B` is resolved by `SKIRMISH_CHOOSE_MAP_0X6B_PREVIEW_REFRESH_GHIDRA_REPORT.md`: passive `0x553` row highlight and `0x6EB` category rebuild do not refresh the preview; Use Map commit plus parent return is the normal preview replacement boundary.
- Exact `>800` modal background pixels require runtime screenshot validation. Binary load path only loads `DAT_00B0FAB8` when screen width is exactly `800`, while `0x6B` still participates in fullscreen shell move.
- Create Random Map `0x583` button dispatch is verified, but downstream random-map UI/generator visuals are outside this report.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Modal create/show/pump | verified | `0x005E68A0`, assembly `0x005E68B7..0x005E690F` | none |
| Modal background/palette load | verified | `0x0072D120`, `0x0072D170`, `0x0072D210`, strings `0x00844FE0/0x00844FC4` | runtime `>800` screenshot validation |
| Dialog id background binding | verified | `0x0060CF00` branch `iVar3 == 0x6B` | none |
| Fullscreen shell parent move | verified | `0x00622820`, `0x0060C540` | none |
| Resource/control inventory | verified by prior doc and spot checks | `SKIRMISH_CHOOSE_MAP_MODAL_VISUAL_CONTROL_LAYOUT_GHIDRA_REPORT.md`, `0x005E7160` | none for inventory |
| Button command/result wiring | verified | `0x005E69C2..0x005E69EC`, `0x005E6B63`, `0x005E7160` | random-map downstream out of scope |
| Listbox owner-draw classification | verified | `0x0060FC18..0x0060FC29`; resource style `0x151`; `0x005E7160` listbox messages | exact row paint deferred |
| Parent hide/show/invalidation boundary | verified | `0x006ACEE0`; sibling preview/accept reports | none for immediate modal boundary |
| Current Rust integration | verified by code scan | `src/app.rs`, `src/ui/skirmish_shell`, `src/app_skirmish_shell_render.rs`, `src/render/skirmish_shell_chrome.rs` | implementation |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-1 - Is Choose Map a separate shell modal? -> Yes, dialog id 0x6B is created and pumped by 0x005E68A0.` (evidence: `0x005E68B7..0x005E690F`)
- `[RESOLVED] OQ-2 - Is MnScrnLCustomizeBattle active in standard YR? -> Yes for dialog 0x6B, conditional SHP load at width 800; PAL load is attempted by 0x0072D120.` (evidence: `0x0072D120`, strings `0x00844FE0/0x00844FC4`)
- `[RESOLVED] OQ-3 - Does 0x6B reuse base setup 0x102 background binding? -> No; 0x0060CF00 has a distinct 0x6B branch writing DAT_00B0FAB8, while 0x102 uses DAT_00B0FA18.` (evidence: `0x0060CF00`)
- `[RESOLVED] OQ-4 - Are modal buttons ordinary owner-draw shell push buttons? -> Yes; Button style low bits select OwnerDraw_Button_00612B70 and callback dispatch handles 0x6C5/0x583/0x5C0.` (evidence: resource style, `0x0060FE58`, `0x005E69C2..0x005E69EC`)
- `[RESOLVED] OQ-5 - Are modal map/mode controls listboxes or combos? -> Listboxes; class/style and accept messages use LISTBOX / LB_GETCURSEL / LB_GETITEMDATA.` (evidence: resource `0x6B`, `0x0060FC18`, `0x005E7160`)
- `[RESOLVED] OQ-6 - Does parent setup remain visible behind modal? -> No; parent hides before modal and shows after return.` (evidence: `0x006ACEE0`, sibling modal flow report)
- `[RESOLVED] OQ-7 - Exact selected row fill/text in OwnerDraw_ListBox_00618D40.` Follow-up `SKIRMISH_OWNERDRAW_LISTBOX_00618D40_ROW_PAINT_GHIDRA_REPORT.md` verifies real Choose Map listboxes use font+2 row height, full-row selected fill, `+2` text inset, and 20 px scrollbar content shrink.
- `[RESOLVED] OQ-8 - Does chooser 0x468 repaint live preview on row highlight before Use Map? -> No for normal `0x6B` browsing. `SKIRMISH_CHOOSE_MAP_0X6B_PREVIEW_REFRESH_GHIDRA_REPORT.md` verifies passive map-list `0x553` highlight has no preview-refresh branch, category `0x6EB` rebuild does not reload preview, and normal preview replacement occurs after Use Map commits and parent `0x102` refreshes selected map state.`

## 9. Stale Docs / Follow-up Docs

- `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_SHELL_RETAIL_ASSETS_GHIDRA_REPORT.md`: replace the Section 8 caution sentence beginning "`MnScrnLCustomizeBattle.shp/.PAL` sounds relevant by name..." with: "`MnScrnLCustomizeBattle.shp/.PAL` is not a base offline Skirmish setup `0x102` background, but it is verified for the Choose Map modal dialog `0x6B`: `0x0072D120` loads the SHP at exactly 800-wide screens and loads the PAL/convert state, and `0x0060CF00` binds `DAT_00B0FAB8`/`FUN_0072D210()` for dialog id `0x6B`."
- `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_CHOOSE_MAP_MODAL_VISUAL_CONTROL_LAYOUT_GHIDRA_REPORT.md`: no replacement needed; this report agrees with its modal asset/control boundary and narrows only the Rust-facing integration delta.

## Sources

- Ghidra read-only string search: `MnScrnLCustomizeBattle.PAL @ 0x00844FC4`, `MnScrnLCustomizeBattle.shp @ 0x00844FE0`.
- Ghidra read-only decompile: `FUN_005e68a0`, `FUN_0060cf00`, `FUN_0072d120`, `FUN_0072d170`, `FUN_0072d210`, `FUN_00622820`, `FUN_0060c540`, `FUN_0060f9a0`, `FUN_005e7160`, `FUN_006acee0`.
- Ghidra read-only assembly context: `0x005E68A0..0x005E690F`, `0x0072D120..0x0072D166`, `0x0060FC18`, `0x0060FE58`, `0x005E69C2`, `0x005E69D3`, `0x005E6B63`, `0x005E6EA6`, `0x005E6F17`.
- Existing docs read/reconciled: `SKIRMISH_CHOOSE_MAP_MODAL_VISUAL_CONTROL_LAYOUT_GHIDRA_REPORT.md`, `SKIRMISH_CHOOSE_MAP_MODAL_FLOW_GHIDRA_REPORT.md`, `SKIRMISH_CHOOSE_MAP_PREVIEW_REFRESH_FUN_006ACEE0_GHIDRA_REPORT.md`, `SKIRMISH_SHELL_RETAIL_ASSETS_GHIDRA_REPORT.md`, `SKIRMISH_STATIC_TEXT_SUBCLASS_THUNK_00610CA0_GHIDRA_REPORT.md`, `FUN_0060F9A0_OWNERDRAW_SUBCLASS_SETUP_GHIDRA_REPORT.md`.
- Rust surfaces scanned: `src/app.rs`, `src/ui/skirmish_shell/layout.rs`, `src/ui/skirmish_shell/state.rs`, `src/app_skirmish_shell_render.rs`, `src/render/skirmish_shell_chrome.rs`.
