# Skirmish 0x102 Status Help Full Mapping Current Rust - Ghidra Research Report

**Address(es):** `0x00622B50`, `0x006040B0`, `0x006AE3F0`, `0x00603F00`, `0x004E3830`, `0x004E4170`, `0x004E38A0`, `0x004E4230`, `0x004E42A0`, `0x004E4EC0`, `0x004E4F30`, `0x004E5900`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** standard offline YR parent Skirmish setup dialog `0x102` status/help child `0x695`: update timing, source precedence, exact static `STT:*` fallback mapping, item-specific override cases relevant to current Rust, blank fallback, current Rust implementation status, and the boundary proof that Choose Map `0x6B` uses a separate dialog-id mapping path.
**Non-Scope:** full Choose Map `0x6B` row/status implementation beyond the parent-vs-modal boundary, validation modal native status behavior, online host/guest dialogs, full CSF text content, runtime screenshot capture, and implementing Rust.
**Confidence:** High for parent `0x102` source order, static fallback map, blank fallback, Choose Map separation, and current Rust deltas; Medium for full item-specific country/color/start tooltip key names because this slot verified the binary helper chain and string clusters but did not build a complete numeric string-id-to-key table for every country/color entry.
**Active in YR:** Yes for standard offline Skirmish `0x102`.

## 0. Working Notes Gate

- Target question: After current Rust added `status_help_text`, `hovered_shell_control`, and `status_help_key_for_hover`, does it match standard YR parent Skirmish `0x102` hover/status-help behavior?
- Non-goals: Do not re-audit `0x695` geometry, do not cover full Choose Map `0x6B` row/status behavior beyond the boundary check, do not patch Rust, do not mutate Ghidra.
- Evidence needed to mark COMPLETE: active common-handler path into `0x695`, source precedence proof, static `FUN_006040B0` `0x102` mapping, item override proof for open combo rows, blank fallback proof, modal/dialog-id separation proof for Choose Map `0x6B`, Rust scan of state/app/render/layout surfaces, and explicit stale-doc wording.
- Stop conditions: stop when all scoped source-order and mapping questions are resolved or deferred, and when current Rust mismatches can be stated as implementation handoff items.

## 1. Overview

The parent Skirmish status strip is updated by the common shell parent handler during `WM_NCHITTEST`, not by Skirmish-specific tick/render code. The handler clears the string holder, hit-tests the child under the cursor, tries child `0x4E8` plus parent `0x4E9` item text, then falls back to `FUN_006040B0(parent, hovered_child)` for a static `STT:*` key and finally sends message `0x4B2` to child `0x695`.

Choose Map status-help is separate at the dialog-id switch level: the same static helper has a distinct `0x6B` branch with `STT:Scenario*` keys, while parent Skirmish uses the `0x102` branch with `STT:Skirmish*` keys. Current Rust mirrors that boundary by routing mouse movement to `update_choose_map_modal_status_help` while `choose_map_modal` is open, and otherwise to `update_skirmish_shell_status_help`.

Current Rust now has the basic surface: layout rect, state string, renderer, mouse-move wiring, CSF lookup, and tests. It is not complete for full parent `0x102` parity: flag picture controls `0x6DA..0x6E1` and right-panel statics `0x6EC/0x5A8` are not hover targets, and non-AI open dropdown row hovers currently use generic combo text where the binary first attempts item-specific text for side/country, color, and start rows.

## 2. Key Controls And Mapping

| Control IDs | Binary status text source | Current Rust status | Active in YR |
|---|---|---|---|
| `0x695` | no static case; blank fallback | self hover returns `None`, matches | Yes |
| `0x6A0` | `STT:SkirmishEditPlayer` | implemented | Yes |
| `0x50B/0x50E/0x516/0x51A/0x51B/0x51C/0x51D` AI combo faces | `STT:SkirmishComboAIPlayer` | implemented | Yes |
| same AI combo open rows | item data `-1/2/1/0` -> `STT:PlayerNone/PlayerDumbAI/PlayerSmartAI/PlayerGeniusAI` before generic fallback | implemented | Yes |
| `0x6DA..0x6E1` flag pictures | `STT:SkirmishPictureFlag` | missing hover target | Yes |
| `0x6A1/0x510/0x513/0x51E/0x514/0x51F/0x520/0x521` side/country combo faces | `STT:SkirmishComboCountry` | implemented | Yes |
| same side/country open rows | item-specific country/random/observer strings through `FUN_004E4170 -> FUN_004E38A0` before generic fallback | mismatch: Rust uses generic combo key for non-AI items | Conditional on open dropdown |
| `0x6A2/0x522..0x528` color combo faces | `STT:SkirmishComboColor` | implemented | Yes |
| same color open rows | item-specific color/random/observer strings through `FUN_004E4E20 -> FUN_004E42A0` before generic fallback | mismatch: Rust uses generic combo key for non-AI items | Conditional on open dropdown |
| `0x6A3..0x6A8/0x6AA/0x6AB` start combo faces | `STT:HostComboStart` | implemented | Yes |
| same start open rows | start item helper `FUN_004E5900`, then `FUN_004E4F30` (`STT:HostComboStart`) | broadly same generic key | Conditional on open dropdown |
| `0x76D..0x774` team combo faces/rows | `STT:HostComboTeam` | implemented | Yes |
| `0x529/0x511/0x50C` trackbars | `STT:SkirmishSliderSpeed/Credits/Unit` | implemented | Yes |
| `0x5AA/0x617/0x5C0` buttons | `STT:SkirmishButtonChooseMap/StartGame/Back` | implemented | Yes |
| `0x693/0x54E/0x69A/0x69D/0x696` checkboxes | `STT:SkirmishCBoxRedeploys/ShortGame/SWAllowed/BuildOffAlly/Crates` | implemented | Yes |
| `0x468` map preview | `STT:SkirmishMapThumbnail` | implemented | Yes |
| `0x6EC` right-panel game type label | `STT:SkirmishLabelGameType` | missing hover target | Yes |
| `0x5A8` right-panel scenario/map label | `STT:SkirmishLabelScenario` | missing hover target | Yes |
| parent/background or unknown child | empty wide string `0x00887734` | implemented as empty when no target/key | Yes |

## 3. Core Logic

### 3.1 Update Timing And Source Order

Active in YR: Yes. `FUN_006AE3F0` calls `FUN_00622B50` first (`0x006AE40A`) for standard `0x102` messages. In `FUN_00622B50`, the `WM_NCHITTEST (0x84)` branch gets child `0x695` (`0x00622CCB`), converts the cursor point, calls `ChildWindowFromPointEx(parent, point, 1)`, and clears the string holder before status resolution.

If a child is found, the handler sends `0x4E8` to the hovered child (`0x00622D4B..0x00622D54`). It then calls `FUN_00603F00`, which sends parent/child status message `0x4E9` using the child and returned hit/item value. If that holder is still empty, the handler sends parent `0x4E9` again with item/index `-1` (`0x00622DB0..0x00622DC4`). If still empty, it calls `FUN_006040B0` (`0x00622E1D..0x00622E38`). If that returns null, it uses empty string `0x00887734` (`0x00622E40` in the prior report). It always converts the final holder to wide text and sends `SendMessageA(status_hwnd, 0x4B2, 0, wide_text)` (`0x00622E6D..0x00622E83`).

Current Rust updates from `src/app.rs:1216` mouse move events, not an explicit native `WM_NCHITTEST` equivalent. That is likely sufficient for ordinary visible cursor movement, but it can miss native-equivalent status refreshes caused by hit-test queries without cursor movement.

### 3.2 Static Fallback Mapping

Active in YR: Yes. `FUN_006040B0` reads the parent dialog id from the shell record and switches on dialog `0x102`. The exact static `0x102` returns observed in decompile are:

- `0x6A0 -> STT:SkirmishEditPlayer`
- `0x50B/0x50E/0x516/0x51A/0x51B/0x51C/0x51D -> STT:SkirmishComboAIPlayer`
- `0x6DA..0x6E1 -> STT:SkirmishPictureFlag`
- `0x6A1/0x510/0x513/0x51E/0x514/0x51F/0x520/0x521 -> STT:SkirmishComboCountry`
- `0x6A2/0x522/0x523/0x524/0x525/0x526/0x527/0x528 -> STT:SkirmishComboColor`
- `0x6A3/0x6A4/0x6A5/0x6A6/0x6A7/0x6A8/0x6AA/0x6AB -> STT:HostComboStart`
- `0x76D/0x76E/0x76F/0x770/0x771/0x772/0x773/0x774 -> STT:HostComboTeam`
- `0x529 -> STT:SkirmishSliderSpeed`
- `0x511 -> STT:SkirmishSliderCredits`
- `0x50C -> STT:SkirmishSliderUnit`
- `0x5AA -> STT:SkirmishButtonChooseMap`
- `0x693 -> STT:SkirmishCBoxRedeploys`
- `0x54E -> STT:SkirmishCBoxShortGame`
- `0x69A -> STT:SkirmishCBoxSWAllowed`
- `0x69D -> STT:SkirmishCBoxBuildOffAlly`
- `0x696 -> STT:SkirmishCBoxCrates`
- `0x468 -> STT:SkirmishMapThumbnail`
- `0x6EC -> STT:SkirmishLabelGameType`
- `0x5A8 -> STT:SkirmishLabelScenario`
- `0x617 -> STT:SkirmishButtonStartGame`
- `0x5C0 -> STT:SkirmishButtonBack`

No `0x695` case exists in the `0x102` branch. Hovering `0x695`, an unmapped child, or a blank area resolves to empty text.

### 3.3 Item-Specific Overrides

Active in YR: Conditional, only when the hit-tested child returns a non-`-1` item index through `0x4E8`.

`FUN_006AE3F0` handles parent message `0x4E9`. For AI row-state combo controls, it reads item data and maps:

| Item data | Status text | Evidence | Active in YR |
|---:|---|---|---|
| `-1` | `STT:PlayerNone` | `0x006AE4A2..0x006AE4B8`; prior AI row report | Yes |
| `2` | `STT:PlayerDumbAI` | `0x006AE4BA..0x006AE4CE`; prior AI row report | Yes |
| `1` | `STT:PlayerSmartAI` | `0x006AE4D0..0x006AE4E4`; prior AI row report | Yes |
| `0` | `STT:PlayerGeniusAI` | `0x006AE4E6..0x006AE4FA`; prior AI row report | Yes |

For side/country combo controls, `FUN_006AE3F0` calls `FUN_004E3830` to recognize the control id, `FUN_004E4170` to read item data, and `FUN_004E38A0` to load item-specific strings for `-3`, `-2`, and `0..9`. Search strings show the relevant string cluster includes `STT:PlayerSideObserver`, `STT:PlayerSideRandom`, and country-specific `STT:PlayerSide*` keys. Current Rust does not model this item-specific status text.

For color combo controls, `FUN_004E4230` recognizes the control id, `FUN_004E4E20` reads selected/hovered item data, and `FUN_004E42A0` maps `-2` and `0..8` to item-specific color status strings. Search strings show the relevant cluster includes `STT:PlayerColorRandom`, `STT:PlayerColorGold`, `STT:PlayerColorRed`, `STT:PlayerColorBlue`, `STT:PlayerColorGreen`, `STT:PlayerColorOrange`, `STT:PlayerColorSkyBlue`, `STT:PlayerColorPurple`, `STT:PlayerColorPink`, and `STT:PlayerColorObserver`. Current Rust does not model this item-specific status text.

For start-position combo controls, `FUN_004E4EC0` recognizes `0x6A3..0x6AB`, `FUN_004E5900` reads item data, and `FUN_004E4F30` loads `STT:HostComboStart`. This is effectively the same generic start-combo status key current Rust uses.

For team combo controls, no item-specific branch was observed in `FUN_006AE3F0`; `FUN_006040B0` falls back to `STT:HostComboTeam`.

### 3.4 Choose Map Modal Boundary

Active in YR: Yes. `FUN_006040B0` does not reuse the parent `0x102` Skirmish mapping for Choose Map. After the `0x102` branch, it has a separate `iVar4 == 0x6B` branch mapping `0x6EB -> STT:ScenarioListGameType`, `0x553 -> STT:ScenarioListMaps`, `0x468 -> STT:ScenarioMapThumbnail`, `0x6C5 -> STT:ScenarioButtonUseMap`, `0x583 -> STT:ScenarioButtonRandom`, and `0x5C0 -> STT:ScenarioButtonCancel`; unmatched children return null and therefore fall through the common blank fallback.

Current Rust is structurally aligned for this boundary. `handle_skirmish_shell_mouse_move` routes to `update_choose_map_modal_status_help` when `state.skirmish_shell_state.choose_map_modal.is_some()`, and `hovered_shell_control` returns `None` while the modal is open. Rust also adds a mode-row tooltip override from the selected `SkirmishGameMode.tooltip_key`; that is compatible with native mode-row dynamic status behavior documented by the Choose Map status recheck, but the full modal row contract remains covered by the separate Choose Map report.

## 4. INI Keys

No scoped INI keys control parent `0x102` status-help text. The behavior is resource/control-id/CSF driven. No `rules.ini`, `rulesmd.ini`, `art.ini`, or `artmd.ini` reader is part of this slice.

## 5. Current Rust Implementation Status

| Rust surface | Status | Evidence |
|---|---|---|
| `SkirmishShellLayout::status_help` | implemented with verified bottom-left rect | `src/ui/skirmish_shell/layout.rs:181`, `:483`, `:649`, `:1076` |
| `SkirmishShellState::status_help_text` | implemented; blank default, setter/clear helpers | `src/ui/skirmish_shell/state.rs:842`, `:917`, `:926` |
| Parent status rendering | implemented, draws only when non-empty | `src/app_skirmish_shell_render.rs:2069`, `:2172` |
| Parent mouse-move wiring | implemented; uses hovered target -> key -> CSF localized text -> state | `src/app.rs:982`, `:991`, `:1008`, `:1216` |
| Modal suppression | parent `hovered_shell_control` returns `None` while Choose Map or validation modal is open; app separately routes Choose Map status when `choose_map_modal` exists and clears on validation modal | `src/ui/skirmish_shell/state.rs:1331`; `src/app.rs:1010`, `:1216` |
| Choose Map status boundary | implemented as a separate modal resolver before parent hover resolution; includes `STT:Scenario*` fallback plus mode-row tooltip override | `src/app.rs:1021`, `:1043`, `:1216`; `src/ui/skirmish_shell/state.rs:1480`, `:1510` |
| Static fallback map | mostly implemented for player edit, preview, buttons, checkboxes, trackbars, combo faces, team/start, AI item rows | `src/ui/skirmish_shell/state.rs:1390` |
| Missing static hover targets | flags `0x6DA..0x6E1`, `0x6EC` game-type label, and `0x5A8` scenario label are not represented as `SkirmishHoverTarget` variants | `SkirmishHoverTarget` at `src/ui/skirmish_shell/state.rs:91`; `hovered_shell_control` at `:1331` |
| Non-AI dropdown item-specific status | incomplete: non-AI `ComboItem` returns generic combo key | `src/ui/skirmish_shell/state.rs:1428`, `:1433` |

## 6. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `0x102` common handler invocation | verified | `FUN_006AE3F0`, assembly `0x006AE40A` | none |
| `0x695` lookup and final update | verified | `FUN_00622B50`, assembly `0x00622CCB`, `0x00622E6D..0x00622E83` | none |
| child `0x4E8` first source | verified | `FUN_00622B50`, assembly `0x00622D4B..0x00622D54` | none |
| child/parent `0x4E9` helper | verified | `FUN_00603F00` decompile | none |
| parent `0x4E9` second pass with `-1` | verified | `FUN_00622B50`, assembly `0x00622DB0..0x00622DC4` | none |
| static `0x102` `STT:*` map | verified | `FUN_006040B0`; `search_strings STT:Skirmish` | none |
| blank fallback | verified | `FUN_00622B50`; prior assembly `0x00622E40..0x00622E83`; no `0x695` case in `FUN_006040B0` | none |
| Choose Map modal/static status separation | verified | `FUN_006040B0` `iVar4 == 0x6B`; current Rust `update_choose_map_modal_status_help` | full modal row/status behavior belongs to the separate Choose Map report |
| AI row item status | verified | `FUN_006AE3F0`; prior AI row report | none |
| side/country item status | touched-not-exhausted | `FUN_004E3830`, `FUN_004E4170`, `FUN_004E38A0`; string search | exact numeric id -> every country key table if needed |
| color item status | touched-not-exhausted | `FUN_004E4230`, `FUN_004E4E20`, `FUN_004E42A0`; string search | exact numeric id -> every color key table if needed |
| start item status | verified | `FUN_004E4EC0`, `FUN_004E5900`, `FUN_004E4F30`; prior start-position reports | none |
| team item status | verified as generic fallback only | `FUN_006AE3F0`, `FUN_006040B0` | none |
| current Rust parent status surfaces | verified | Codegraph + `rg`/file reads in Sources | implementation deltas remain |

## 7. Open Questions - Final State

- `[RESOLVED] OQ-001 - Is `0x102` status-help active in standard YR? -> Yes, `0x006AE3F0` calls `FUN_00622B50` before specific handling.` (evidence: `0x006AE40A`)
- `[RESOLVED] OQ-002 - What event updates `0x695`? -> Parent `WM_NCHITTEST (0x84)` in the common shell handler.` (evidence: `FUN_00622B50`)
- `[RESOLVED] OQ-003 - Is `0x695` updated through dynamic text message? -> Yes, final send is `SendMessageA(status_hwnd,0x4B2,0,wide_text)`.` (evidence: `0x00622E6D..0x00622E83`)
- `[RESOLVED] OQ-004 - What is source precedence? -> child `0x4E8`/parent `0x4E9` item text, second parent `0x4E9` with `-1`, static `FUN_006040B0`, then empty string.` (evidence: `FUN_00622B50`, `FUN_00603F00`)
- `[RESOLVED] OQ-005 - Does `0x695` self-hover map to text? -> No, no `0x695` branch exists in `FUN_006040B0`; blank fallback applies.` (evidence: `FUN_006040B0`)
- `[RESOLVED] OQ-006 - Is current Rust blank fallback present? -> Yes, missing target/key localizes to empty and renderer skips empty text.` (evidence: `src/app.rs:991..1008`, `src/app_skirmish_shell_render.rs:2172`)
- `[RESOLVED] OQ-007 - Are static button/checkbox/slider/preview mappings present in Rust? -> Yes for currently represented hover targets.` (evidence: `src/ui/skirmish_shell/state.rs:1390..1433`)
- `[RESOLVED] OQ-008 - Are flag picture/static label mappings present in Rust? -> No hover targets exist for flags or right-panel game/map labels.` (evidence: `src/ui/skirmish_shell/state.rs:91`, `:1331`)
- `[RESOLVED] OQ-009 - Does binary have item-specific AI row status? -> Yes, item data `-1/2/1/0` maps to `STT:PlayerNone/DumbAI/SmartAI/GeniusAI`.` (evidence: `FUN_006AE3F0`; prior AI row report)
- `[RESOLVED] OQ-010 - Does current Rust match AI item rows? -> Yes.` (evidence: `src/ui/skirmish_shell/state.rs:1430..1432`, `:1503`)
- `[RESOLVED] OQ-011 - Does binary have side/color item-specific status? -> Yes, side and color helper chains load item-specific strings before generic fallback.` (evidence: `FUN_004E3830`, `FUN_004E4170`, `FUN_004E38A0`, `FUN_004E4230`, `FUN_004E42A0`)
- `[RESOLVED] OQ-012 - Does current Rust match side/color open-row status? -> No, non-AI `ComboItem` falls back to generic combo key.` (evidence: `src/ui/skirmish_shell/state.rs:1433`)
- `[RESOLVED] OQ-013 - Is start open-row status effectively generic? -> Yes, helper reaches `FUN_004E4F30` / `STT:HostComboStart`.` (evidence: `FUN_004E4EC0`, `FUN_004E5900`, `FUN_004E4F30`)
- `[RESOLVED] OQ-014 - Are INI defaults involved? -> No, no scoped INI reader participates.` (evidence: binary function set and source scan)
- `[RESOLVED] OQ-015 - Is Choose Map status mapping separate from parent `0x102`? -> Yes, `FUN_006040B0` has a separate `iVar4 == 0x6B` branch with `STT:Scenario*` keys, and current Rust routes `choose_map_modal` mouse movement to a separate modal resolver before parent hover resolution.` (evidence: `FUN_006040B0`; `src/app.rs:1021..1065`, `src/app.rs:1216..1240`)
- `[DEFERRED] OQ-016 - Exact numeric string ID to every side/color status key.` (category: `bounded-cost-too-high`; reason: helper and string cluster prove item-specific behavior and Rust delta; full table is only needed for the eventual patch; next-step-if-pursued: build a GDlgSupp string-id table for `0xED..0x103` and `0x1C1..0x1D3`)
- `[DEFERRED] OQ-017 - Native status refresh without mouse movement.` (category: `needs-runtime-debugger`; reason: static binary proves `WM_NCHITTEST`; whether retail produces visible updates on all stationary cases needs runtime message trace; next-step-if-pursued: breakpoint `0x00622B50` and move/show dialogs with stationary cursor)

## 8. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Parent `0x102` status update uses child/item text first, static `STT:*` fallback second, then blank; `0x695` itself maps blank | `FUN_00622B50`, `FUN_00603F00`, `FUN_006040B0`; `0x00622E6D..0x00622E83` | mostly implemented | `src/app.rs`, `src/ui/skirmish_shell/state.rs`, `src/app_skirmish_shell_render.rs` | Keep empty default and self-hover blank, update localized text from hover source, render only non-empty | Hover Start -> localized Start help; hover blank/status strip -> empty; move away -> clears | Do not render permanent "Status"/visible labels; proposed test `skirmish_status_help_self_and_blank_hover_clear_text` |
| Choose Map `0x6B` uses a separate static status branch with `STT:Scenario*`, not parent `STT:Skirmish*` keys | `FUN_006040B0` `iVar4 == 0x6B`; `STT:Scenario*` string refs | none for boundary; full modal row contract covered elsewhere | `src/app.rs`, `src/ui/skirmish_shell/state.rs` | Keep modal hover routing separate and block parent hover while Choose Map is open | Open Choose Map, hover Use Map/Cancel/Random/map list/status strip, then close modal and hover parent Choose Map button; modal keys and parent key differ correctly | Do not run parent `0x102` hover resolver under the modal; proposed test `choose_map_status_help_uses_modal_resolver_not_parent_mapping` |
| `FUN_006040B0` maps flag pictures `0x6DA..0x6E1` to `STT:SkirmishPictureFlag` and right-panel statics `0x6EC/0x5A8` to label help keys | `FUN_006040B0`; strings `0x00835400`, `0x0083549C`, `0x00835480` | missing | `SkirmishHoverTarget`, `hovered_shell_control`, `status_help_key_for_hover` | Add hover targets for `layout.flags`, `layout.right_panel_text.game_type`, and `layout.right_panel_text.map_label` | Hover any flag shows `STT:SkirmishPictureFlag`; hover game type label shows `STT:SkirmishLabelGameType`; hover map label shows `STT:SkirmishLabelScenario` | Do not limit status help to interactive controls only; proposed test `skirmish_status_help_includes_flag_and_right_panel_static_targets` |
| Open side/country and color dropdown rows can return item-specific status text before generic combo fallback | `FUN_006AE3F0`; `FUN_004E3830 -> FUN_004E4170 -> FUN_004E38A0`; `FUN_004E4230 -> FUN_004E42A0`; string clusters `STT:PlayerSide*`, `STT:PlayerColor*` | mismatch | `status_help_key_for_hover` or a richer status resolver returning localized text, not only static keys | Resolve country/random/observer and color/random item status for `ComboItem` rows before generic `STT:SkirmishComboCountry/Color` | Open Side dropdown and hover Random/Country row: status changes to item-specific side text, not generic combo help; open Color dropdown and hover color row: item-specific color text | Do not use visible row labels as help text; proposed test `skirmish_status_help_dropdown_side_color_items_use_item_specific_stt` |

## 9. Negative Facts / Do Not Do

- Do not treat `0x695` as a mapped tooltip control. Active in YR: Yes. Evidence: no `0x695` branch in `FUN_006040B0`; blank fallback.
- Do not restrict parent `0x102` status help to interactive widgets. Active in YR: Yes. Evidence: `FUN_006040B0` maps flag pictures and static right-panel labels.
- Do not use generic `STT:SkirmishComboCountry`/`STT:SkirmishComboColor` for every open dropdown row. Active in YR: Conditional on open dropdown. Evidence: `0x4E8 -> 0x4E9` item path and side/color helper chains.
- Do not update `0x695` from INI data or map/game names directly. Active in YR: Yes. Evidence: status path uses shell messages and CSF/string-table keys.
- Do not reuse the parent Skirmish `0x102` `STT:Skirmish*` resolver for Choose Map `0x6B`. Active in YR: Yes. Evidence: `FUN_006040B0` has a separate `iVar4 == 0x6B` branch returning `STT:Scenario*` strings.
- Do not let a validation modal keep stale parent status text. Active in YR: inferred from modal ownership; current Rust clears on validation modal. Evidence for parent path: `hovered_shell_control` blocks status when validation modal exists; validation modal is outside this slice.

## 10. Remaining Uncertainty

- Exact numeric string-id table for every side/country and color item status key remains deferred; the binary helper chain and string clusters are enough to prove current Rust's generic non-AI item fallback is wrong.
- Runtime-only message cadence for stationary cursor cases remains unverified. The binary update event is `WM_NCHITTEST`; current Rust mouse-move wiring matches ordinary movement but not necessarily every native hit-test refresh.
- Full Choose Map mode-row dynamic status behavior remains owned by `SKIRMISH_CHOOSE_MAP_0X6B_STATUS_HOVER_MAPPING_CURRENT_RUST_RECHECK_GHIDRA_REPORT.md`; this report only proves the parent-vs-modal mapping boundary.

## 11. Stale Docs / Replacement Wording

- Replace stale current-Rust wording in `SKIRMISH_STATUS_CHILD_0X695_TEXT_SOURCE_GHIDRA_REPORT.md` that says Rust has no status/help strip state with: "Current Rust now has `SkirmishShellLayout::status_help`, `SkirmishShellState::status_help_text`, non-empty status rendering, and app mouse-move wiring. Remaining parent `0x102` gaps are full hover target coverage for flag pictures and right-panel statics, side/color dropdown item-specific status text, and exact native `WM_NCHITTEST` cadence."
- Replace stale current-Rust wording in `SKIRMISH_SHELL_INPUT_FOCUS_MESSAGE_BROAD_RECHECK_GHIDRA_REPORT.md` that says Rust lacks `0x695` hover status with: "Current Rust implements parent status-help plumbing and most static `0x102` mappings, but the mapping is incomplete for flag/right-panel static controls and non-AI side/color dropdown item-specific hovers."

## Sources

- Ghidra decompile: `FUN_00622B50`, `FUN_00603F00`, `FUN_006040B0`, `FUN_006AE3F0`, `FUN_004E3830`, `FUN_004E4170`, `FUN_004E38A0`, `FUN_004E4230`, `FUN_004E4E20`, `FUN_004E42A0`, `FUN_004E4EC0`, `FUN_004E5900`, `FUN_004E4F30`.
- Ghidra assembly context: `0x006AE40A`, `0x00622CCB`, `0x00622D4B..0x00622D54`, `0x00622DB0..0x00622DC4`, `0x00622E1D..0x00622E38`, `0x00622E6D..0x00622E83`.
- Ghidra string search: `STT:Skirmish*` at `0x008353E4..0x008355C8`, `STT:HostComboStart @ 0x00822B90`, `STT:HostComboTeam @ 0x0083377C`, `STT:PlayerSide*`, `STT:PlayerColor*`, `STT:PlayerNone/DumbAI/SmartAI/GeniusAI`.
- Prior docs: `SKIRMISH_STATUS_CHILD_0X695_TEXT_SOURCE_GHIDRA_REPORT.md`, `SKIRMISH_SHELL_INPUT_FOCUS_MESSAGE_BROAD_RECHECK_GHIDRA_REPORT.md`, `SKIRMISH_AI_ROW_STATE_LABELS_AND_ITEM_DATA_GHIDRA_REPORT.md`, `SKIRMISH_START_POSITION_UX_GHIDRA_REPORT.md`, `SKIRMISH_START_POSITION_COMBO_POPULATION_GHIDRA_REPORT.md`, `SKIRMISH_CHOOSE_MAP_0X6B_STATUS_HOVER_MAPPING_CURRENT_RUST_RECHECK_GHIDRA_REPORT.md`.
- Rust scan: `src/ui/skirmish_shell/layout.rs`, `src/ui/skirmish_shell/state.rs`, `src/app.rs`, `src/app_skirmish_shell_render.rs`.
