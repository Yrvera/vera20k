# Skirmish Choose Map Modal Visual Control Layout - Ghidra Report

**Address(es):** `0x005E68A0`, callback entry `0x005E6920`, `0x005E7160`, `0x0060CF00`, `0x00622820`, `0x0060C540`, resource dialog `0x6B`  
**Investigation Mode:** exhaustive-slice scoped to visual/control layout only  
**Claimed Scope:** Choose Map modal dialog/background asset, control inventory and rectangles, listbox controls `0x553`/`0x6EB`, accept/cancel/random-map buttons, preview/static controls, and shell hide/show/repaint visual requirements.  
**Non-Scope:** map filtering predicate beyond control naming, PKT/source order, random-map generator internals, exact list item text construction, and parent setup accept/cancel state rebuild already covered by sibling reports.  
**Confidence:** High for resource template layout, active modal creation, control IDs, button result IDs, background asset selection, and shell fullscreen resize policy. Medium for final pixel-perfect list row internals because the real owner-drawn listbox callback was not fully drained in this slot.  
**Active in YR:** Yes. Evidence: the standard offline Skirmish `0x5AA` branch calls `0x005E68A0`; `0x005E68A0` creates dialog resource `0x6B` with callback entry `0x005E6920`; resource `0x6B` exists in retail `gamemd.exe` `RT_DIALOG` language `0x409`.

## 1. Summary

Retail Choose Map is a full shell modal with the same `533x369` dialog-template frame as the Skirmish setup dialog, not an in-place popup widget or an overlay drawn inside setup. The parent setup shell is hidden before entry; the modal is created from PE `RT_DIALOG` resource `0x6B`, processed by the normal shell owner-draw setup path, moved to fullscreen shell coordinates, shown, and pumped by the modal loop.

The implementation-facing visual model is:

- background/chrome comes from the shell background loader for dialog id `0x6B`, not from Skirmish setup `0x102` assets;
- the modal has two real owner-drawn listboxes: game type/category `0x6EB` and map list `0x553`;
- the modal has right-column owner-drawn buttons `Cancel`, `Use Map`, and `Create Random Map`;
- the modal has the usual right-panel title static `0x694` and preview static `0x468`;
- it does not contain the parent setup's `0x6EC` game-type label or `0x5A8` scenario label.

## 2. Resource Template `0x6B`

Fresh local PE resource extraction from `C:/Users/enok/Documents/Command and Conquer Red Alert II/gamemd.exe` found:

| Field | Value | Evidence | Active in YR |
|---|---:|---|---|
| Resource type/name/lang | `RT_DIALOG`, id `0x6B`, lang `0x409` | PE resource data RVA `0x7EE6D8`, file offset `0x4F26D8`, size `636` | Yes |
| Template kind | `DIALOGEX` | resource signature `0xFFFF` after dialog version | Yes |
| Dialog rect | `(0,0,533,369)` dialog units | resource header | Yes |
| Item count | `11` | resource header | Yes |
| Font | `MS Sans Serif`, 8 pt, weight `0`, italic `0`, charset `1` | resource font block | Yes |
| Style/exstyle | style `0x40000040`, exstyle `0` | resource header | Yes |
| Title/menu/class | all empty | resource variable fields | Yes |

The 11 controls are:

| ID | Rect | Class | Style / exstyle | Title / role | Active in YR |
|---:|---:|---|---|---|---|
| `0x5C0` | `(425,346,108,23)` | `BUTTON` | `0x5000000B` / `0` | `GUI:Cancel` | Yes |
| `0x6C5` | `(425,122,108,23)` | `BUTTON` | `0x5000000B` / `0` | `GUI:UseMap` | Yes |
| `0x583` | `(425,149,108,23)` | `BUTTON` | `0x5000000B` / `0` | `GUI:CreateRandomMap` | Conditional |
| `0x694` | `(425,1,108,10)` | `STATIC` | `0x50020001` / `0` | `GUI:ChooseMap` title | Yes |
| `-1` | `(80,20,257,12)` | `STATIC` | `0x50000201` / `0` | `GUI:SelectEngagement` | Yes |
| `0x6EB` | `(77,78,130,211)` | `LISTBOX` | `0x50000151` / `0` | game type/category list | Yes |
| `0x553` | `(225,78,130,211)` | `LISTBOX` | `0x50000151` / `0` | map list | Yes |
| `-1` | `(77,60,130,10)` | `STATIC` | `0x50000201` / `0` | `GUI:GameType` heading | Yes |
| `-1` | `(225,60,130,10)` | `STATIC` | `0x50000201` / `0` | `GUI:GameMap` heading | Yes |
| `0x695` | `(2,355,303,12)` | `STATIC` | `0x50000200` / `0` | `GUI:Blank` help/status strip | Yes |
| `0x468` | `(428,23,96,69)` | `STATIC` | `0x50000004` / `0x20` | preview placeholder | Yes |

Tiny layout details:

- `0x468` is at X `428`, not the setup dialog's X `429`. Active in YR: Yes; evidence: resource `0x6B` vs `0x102` template extraction.
- The bottom status/help strip `0x695` is width `303`, not setup dialog `0x102` width `410`. Active in YR: Yes; evidence: resource `0x6B`.
- Both listboxes use style low bits `0x151`: notify, owner-draw fixed, has-strings, and no-integral-height. There is no `LBS_SORT` bit. Active in YR: Yes; evidence: resource template plus modal list report's forward-order append.
- The labels `GUI:GameType` and `GUI:GameMap` are static text controls with id `-1`; code should not expect them as stateful/update targets. Active in YR: Yes; evidence: resource template.
- `0x6EB` and `0x553` are real `LISTBOX` controls, not `COMBOBOX` controls. Active in YR: Yes; evidence: resource template and `0x005E7160` using listbox messages `0x188`/`0x199`.

## 3. Modal Entry And Shell Setup

`0x005E68A0` is the active wrapper for this modal:

- calls `0x0072D120` before creation;
- sets `EDX = 0x6B`;
- pushes callback entry `0x005E6920`;
- calls `0x00775700`;
- stores the returned HWND in `DAT_00AC0D40`;
- calls `0x00622820`;
- sends `0x4A9` to the chooser;
- calls `ShowWindow(chooser, 1)`;
- pumps through `0x007759E0(0,1,1)`;
- calls cleanup `0x0072D170` after the modal returns.

Active in YR: Yes. Evidence: Ghidra decompile of `FUN_005e68a0`; assembly at `0x005E68B7..0x005E68C4` (`MOV EDX,0x6B`, `PUSH 0x005E6920`, `CALL 0x00775700`), `0x005E68D0`, `0x005E68E3..0x005E68E9`, `0x005E68F5..0x005E690F`.

`0x00622820` applies the common shell owner-draw setup to the modal. It enumerates children through the same owner-draw/subclass helpers used by shell dialogs, assigns parent background state through `0x0060CF00`, then calls `0x0060C540`. Because dialog id `0x6B` is in the fullscreen-shell set, `0x00622820` moves the parent HWND to `(0,0,g_ScreenWidth,g_ScreenHeight)` and enumerates children through `ResizeShellChildControl_0060C0C0`.

Active in YR: Yes. Evidence: decompile of `0x00622820`; `0x0060C540` returns true for `iVar1 == 0x6B`; `0x0060C4A0`/inline equivalent calls `MoveWindow(parent,0,0,g_ScreenWidth,g_ScreenHeight,0)`.

## 4. Background / Asset Selection

Dialog id `0x6B` has its own shell background pair:

- `0x0072D120` loads `MnScrnLCustomizeBattle.shp` into `DAT_00B0FAB8` only when `g_ScreenWidth == 800`;
- `0x0072D120` always attempts to load `MnScrnLCustomizeBattle.PAL` through `0x0072ADE0`, producing raw/convert state `DAT_00B0FCD0`/`DAT_00B0FCD4`;
- `0x0060CF00`, for dialog id `0x6B`, writes parent state `+0x74 = FUN_0072D210() -> DAT_00B0FCD4`, `+0xE0 = DAT_00B0FB50`, and `+0xE4 = DAT_00B0FAB8`;
- `0x0072D170` frees the SHP/palette/convert state after modal exit.

Active in YR: Yes for dialog `0x6B`; conditional for the `MnScrnLCustomizeBattle.shp` SHP pointer because it is only loaded at width `800`. Evidence: Ghidra decompile/assembly of `0x0072D120`, `0x0072D170`, `0x0072D210`, `0x0060CF00`; memory pointer table `0x00844D64 -> 0x00844FE0 "MnScrnLCustomizeBattle.shp"` and `0x00844D68 -> 0x00844FC4 "MnScrnLCustomizeBattle.PAL"`.

This corrects the visual asset boundary: `MnScrnLCustomizeBattle.*` is not the offline setup dialog `0x102` background, but it is the verified Choose Map modal `0x6B` background/palette path.

## 5. Buttons And Command IDs

The right-column buttons use the same owner-drawn button class/style as the setup shell:

- `0x5C0` Cancel at `(425,346,108,23)` closes with modal result `2`;
- `0x6C5` Use Map at `(425,122,108,23)` calls `0x005E7160`, which commits the selected list items and closes with result `1`;
- `0x583` Create Random Map at `(425,149,108,23)` is present between Use Map and the empty right-column area. Its branch hides the chooser and enters a random-map flow; generator internals are out-of-scope here.

Active in YR: Yes for `0x5C0`/`0x6C5`; Conditional for `0x583` because it requires the player to click Create Random Map. Evidence: resource template; chooser callback assembly `0x005E69C2..0x005E69EC` (`0x6C5`, `0x583`, `0x5C0` dispatch), `0x005E6B63..0x005E6B67` accept call, sibling return-contract report for result values.

Button art implication: because these are `BUTTON` style `0x5000000B` owner-draw controls with 23 dialog-unit height like other right-column shell buttons, they should reuse the shell owner-draw button PCX cap/middle/cap path (`bue_*30` / `bde_*30`) rather than bespoke Choose Map bitmaps. Active in YR: Yes by owner-draw setup path; evidence: `0x00622820` -> `FUN_0060F9A0`, existing `0x00612B70` button callback report.

## 6. Listboxes And Preview/Static Controls

`0x6EB` is the left game-type/category listbox. It is populated during custom init around `0x005E6EA6`: the callback finds the selected mode/category from `DAT_00A8B250`, gets child `0x6EB`, and calls `0x005D6130`.

Active in YR: Yes. Evidence: assembly `0x005E6EA6..0x005E6EDE`, list-population sibling report.

`0x553` is the right map listbox. The callback filters existing scenario records into a temporary vector, then gets child `0x553` and hands the vector to the owner-drawn list backing object. Accept reads `LB_GETCURSEL`/`LB_GETITEMDATA` from `0x553`.

Active in YR: Yes. Evidence: assembly `0x005E6F17..0x005E6F51`; `0x005E7160` decompile uses `SendDlgItemMessageA(DAT_00AC0D40,0x553,0x188/0x199,...)`.

`0x468` is present in the modal at `(428,23,96,69)`. It is the same shell preview placeholder id used in setup/host contexts, but this slot did not verify a chooser-specific preview paint path from `0x005E6920`; only the resource/control presence and common static subclassing were verified. Active in YR: Yes as a control; preview drawing inside the chooser remains Remaining Uncertainty.

`0x694` is a title static `GUI:ChooseMap`, and `0x695` is the bottom help/status static. Static animation classification includes dialog id `0x6B` + control `0x694`, but `0x6B` does not have setup labels `0x6EC`/`0x5A8`. Active in YR: Yes. Evidence: resource template; `0x00602490` decompile includes `iVar5 == 0x6B && id == 0x694`; no `0x6EC`/`0x5A8` resources in `0x6B`.

## 7. Shell Hide/Show And Repaint Requirements

The parent setup shell is visually hidden before this modal is shown, then restored after modal result handling. This report reuses the sibling return-contract proof and adds the modal-side visual setup: the chooser itself is shown with `ShowWindow(...,1)` after the `0x4A9` initialization message and common shell setup.

Active in YR: Yes. Evidence: parent flow report cites setup `ShowWindow(setup,0)` at `0x006AD93C..0x006AD93F` and setup `ShowWindow(setup,5)` after return; modal wrapper shows `DAT_00AC0D40` at `0x005E68F5..0x005E68F8`.

Implementation consequence: do not render the Choose Map modal as a child overlay while the setup controls remain visible/interactive behind it. Treat it as a separate shell screen state using resource `0x6B` layout and modal accept/cancel return semantics.

## 8. Rust Implementation Handoff

Affected Rust surfaces:

- `src/ui/skirmish_shell/state.rs`: add explicit choose-map modal state rather than cycling `selected_map_idx`.
- `src/app_skirmish_shell_render.rs`: add a renderer for dialog resource `0x6B` using shell background assets, two listboxes, right-column buttons, title/status statics, and optional preview placeholder.
- `src/app.rs`: route `ChooseMap` into modal state, make `UseMap` commit and `Cancel` restore according to sibling return-contract docs.
- `src/app_list_maps.rs` / map-menu model: provide listbox rows with item-data-like stable map record identity and selected mode/category identity.

Concrete acceptance/test proposals:

- `choose_map_modal_uses_resource_0x6b_control_rects`: verifies controls `0x6EB`, `0x553`, `0x6C5`, `0x583`, `0x5C0`, `0x694`, `0x468`, and `0x695` are laid out from the `0x6B` resource contract, not copied from setup `0x102`.
- `choose_map_modal_cancel_restores_parent_without_committing_selection`: click Choose Map, change highlighted map, Cancel `0x5C0`; selected setup map and preview remain old.
- `choose_map_modal_use_map_commits_selected_list_item`: click Choose Map, select a map row from `0x553`, Use Map `0x6C5`; setup selection/preview refresh after modal close.
- `choose_map_modal_lists_are_not_sorted_by_display_name`: populate records in source order and verify the modal preserves append order for `0x553`.

## 9. Negative Facts / Do Not Do

- Do not implement Choose Map as in-place cycling. Active in YR: No; evidence: modal wrapper `0x005E68A0` is called from setup `0x5AA`.
- Do not copy setup dialog `0x102` controls wholesale. The modal has 11 controls, no player table, no checkboxes/sliders, no `0x6EC`, and no `0x5A8`. Active in YR: Yes; evidence: resource `0x6B`.
- Do not use `MnScrnLCoopGameSetup.*` as the modal-specific background. That pair is for setup `0x102`; `0x6B` uses `MnScrnLCustomizeBattle.*`. Active in YR: Yes; evidence: `0x0060CF00` branches for ids `0x6B` vs `0x102`.
- Do not use display-name sort for the map listbox. The listbox style lacks `LBS_SORT`, and population appends filtered records in global-record order. Active in YR: Yes; evidence: resource style `0x151`, `0x005E6F17..0x005E6F45`.
- Do not treat `0x6EB` as a dropdown combo. It is a real `LISTBOX` in the modal. Active in YR: Yes; evidence: resource class and `0x005E7160` listbox messages.

## 10. Remaining Uncertainty

- Exact row internal paint for real owner-drawn `LISTBOX` controls `0x6EB`/`0x553` is now resolved by `SKIRMISH_OWNERDRAW_LISTBOX_00618D40_ROW_PAINT_GHIDRA_REPORT.md`: standard rows use font+2 height, full-row selected fill, `+2` text inset, and 20 px scrollbar content shrink.
- `0x468` preview timing while browsing is now resolved by `SKIRMISH_CHOOSE_MAP_0X6B_PREVIEW_REFRESH_GHIDRA_REPORT.md`: normal modal row highlight/category rebuild does not refresh the preview; the chooser repaints current `DAT_00AC1154`, and normal preview replacement happens after Use Map commits and parent `0x102` refreshes selected map state.
- The `>800` background case inherits the known shell uncertainty: `0x0072D120` only loads the alternate SHP at exactly `800`, while fullscreen resizing includes `0x6B`. Runtime screenshot validation is still useful for 1024x768.
- Random-map button `0x583` placement and dispatch are verified, but random-map creation UI/generator visuals are outside this slot.

## 11. Coverage Ledger

| Area | Status | Evidence | Remaining |
|---|---|---|---|
| Dialog resource `0x6B` inventory | verified | PE `RT_DIALOG` extraction from retail `gamemd.exe` | none |
| Modal creation/show/pump | verified | `0x005E68A0` decompile/assembly | none |
| Common owner-draw setup/fullscreen shell move | verified | `0x00622820`, `0x0060C540`, `0x0060C4A0` | none |
| Background asset/palette for `0x6B` | verified | `0x0072D120`, `0x0060CF00`, string pointers | exact >800 runtime pixels deferred |
| Accept/cancel button geometry/result IDs | verified | resource + callback assembly + sibling return report | none |
| Create Random Map button presence | verified | resource + callback branch | generator flow deferred |
| `0x6EB`/`0x553` listbox rect/style | verified | resource + `0x005E6EA6`/`0x005E6F17`/`0x005E7160` | row-paint details deferred |
| Modal preview static `0x468` | control verified | resource + static classification | live chooser preview paint deferred |

## Sources

- Fresh local PE resource extraction: `gamemd.exe` `RT_DIALOG` id `0x6B`, lang `0x409`, data RVA `0x7EE6D8`, file offset `0x4F26D8`, size `636`.
- Fresh read-only Ghidra decompile: `FUN_005e68a0`, `FUN_005e7160`, `FUN_00622b50`, `FUN_0060c540`, `FUN_00608cd0`, `FUN_00609730`, `FUN_00602490`, `FUN_0060a5b0`, `FUN_0072d120`, `FUN_0072d170`, `FUN_0072d210`, `FUN_0060cf00`, `FUN_00622820`, `FUN_0060c4a0`.
- Fresh read-only Ghidra assembly context: `0x005E68A0`, `0x005E6920`, `0x005E69C2`, `0x005E69D3`, `0x005E6B63`, `0x005E6EA6`, `0x005E6F17`, `0x0072D120`, `0x0072D210`.
- Fresh memory/string evidence: pointer table `0x00844D64 -> 0x00844FE0 "MnScrnLCustomizeBattle.shp"`, `0x00844D68 -> 0x00844FC4 "MnScrnLCustomizeBattle.PAL"`.
- Prior reports reconciled: `SKIRMISH_CHOOSE_MAP_MODAL_FLOW_GHIDRA_REPORT.md`, `SKIRMISH_CHOOSE_MAP_MODAL_RETURN_CONTRACT_GHIDRA_REPORT.md`, `SKIRMISH_CHOOSE_MAP_LIST_POPULATION_ORDER_GHIDRA_REPORT.md`, `SKIRMISH_CHOOSE_MAP_PREVIEW_INVALIDATION_GHIDRA_REPORT.md`, `SKIRMISH_SHELL_LAYOUT_ASSETS_GHIDRA_REPORT.md`, `SKIRMISH_SHELL_ACTIVE_RENDER_PATH_LIVE_GHIDRA_REPORT.md`, `SKIRMISH_TEXT_PREVIEW_STATIC_CONTROLS_GHIDRA_REPORT.md`.
