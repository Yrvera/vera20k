# Skirmish Choose Map 0x6B Status Hover Mapping Current Rust Recheck - Ghidra Research Report

**Address(es):** `0x005E68A0`, `0x005E6920..0x005E7041`, `0x00611CBA..0x00611E8B`, `0x005E6E44..0x005E6E97`, `0x006040B0`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** Re-check Choose Map dialog `0x6B` status/help hover behavior against current Rust after parent `0x695` work: exact `0x6B` control/list-row to status text mapping, blank/sticky fallback, parent `0x695` reuse/suppression, and Rust handoff.  
**Non-Scope:** Choose Map modal visual/chrome geometry except where hit testing affects status, keyboard/default dismissal, random-map generation, Start validation modal, parent `0x102` full mapping, Rust implementation, INI edits, and Ghidra mutation.  
**Confidence:** High for active `0x6B` path, common subclass status write, mapped `STT:Scenario*` keys, mode-list row override, modal status child rect/current Rust presence, parent-stale-text suppression, and missing modal resolver. Medium for true blank parent-background behavior because static code proves child/unmapped clearing but not parent-background clearing.  
**Active in YR:** Yes for standard offline Yuri's Revenge Skirmish Choose Map modal.

## 0. Investigation Gate

**Target question:** After recent parent status strip work, what exactly must current Rust implement for Choose Map `0x6B` status hover text, and does the modal reuse/suppress parent `0x695` behavior correctly?

**Non-goals:** Do not rediscover settled `0x6B` composition, modal buttons, map-list population, preview refresh, keyboard dismissal, random map flow, broad parent `0x102` status mapping, or any Rust patch.

**Evidence needed to mark COMPLETE:** Prior status-hover report read; current Rust scan for `choose_map_modal`, `status_help_text`, mouse move, modal layout, and `STT:Scenario*`; fresh Ghidra spot-check of active `0x6B` creation/subclass setup, `WM_MOUSEMOVE -> 0x695` status write, `0x4E9` mode-row override, and `FUN_006040B0` `0x6B` key branch; final handoff with tests.

**Stop conditions:** Save exactly this report; update only `.swarm-claims.md` besides this report; do not modify Rust/INI/in-repo docs/Ghidra; leave unresolved parent-background sticky-vs-clear as runtime uncertainty unless static proof is found.

Duplication decision: Partial high-confidence report exists (`SKIRMISH_CHOOSE_MAP_0X6B_STATUS_HOVER_MAPPING_GHIDRA_REPORT.md`), so this recheck scopes to current Rust delta plus fresh spot-checks instead of re-covering the full modal.

## 1. Overview

Native Choose Map `0x6B` owns its own status/help static `0x695` inside the modal. It is not the parent Skirmish setup strip: setup `0x102` is hidden before the modal is shown, and the modal's common shell subclass writes to the modal's child `0x695`.

Current Rust now has both parent and Choose Map `status_help` rects and one shared `SkirmishShellState::status_help_text`, but while `choose_map_modal` is open mouse move clears that text and returns. The visible strip exists; the native `STT:Scenario*` resolver and mode-row override are still missing.

## 2. Class Layout / Key Values

| Value / field | Meaning | Evidence | Active in YR |
|---|---|---|---|
| Dialog id `0x6B` | Choose Map modal | `0x005E68B7` loads `EDX=0x6B`; `0x006AD947` calls `FUN_005E68A0` from Skirmish Choose Map | Yes |
| Callback `0x005E6920` | Dialog procedure bytes | `0x005E68BE` pushes callback pointer; no function boundary needed for scoped branches | Yes |
| Child id `0x695` | Modal status/help static | `GetDlgItem(parent,0x695)` in common subclass path; current Rust layout has `ChooseMapModalLayout.status_help` | Yes |
| Message `0x200` | `WM_MOUSEMOVE` trigger | `0x00611CBA CMP ESI,0x200` | Yes |
| Message `0x4E8` | Ask hovered child for item/index context | `0x00611D1A..0x00611D20` | Yes |
| Message `0x4E9` | Ask parent for item-specific text | `0x00611D3C..0x00611D46`, `0x005E6E44..0x005E6E97` | Yes |
| Message `0x4B2` | Write wide text into owner-draw static | `0x00611E85..0x00611E8B` sends to status hwnd | Yes |
| Empty wide string `0x00887734` | Blank fallback | `0x00611E0D`, `0x00611E50` | Yes |

## 3. Core Logic

### 3.1 Active modal path and status subclass setup

Active in YR: Yes. `FUN_006ACEE0` hides setup and calls `FUN_005E68A0` for command `0x5AA`; fresh assembly spot-check shows `0x006AD947 CALL 0x005E68A0`. `FUN_005E68A0` creates dialog `0x6B` with callback `0x005E6920`, stores `DAT_00AC0D40`, calls `FUN_00622820`, sends `0x4A9`, shows the modal, pumps it, and cleans up. Evidence: fresh decompile of `0x005E68A0`; assembly `0x005E68B7..0x005E68D5`.

Active in YR: Yes. `FUN_00622820` enumerates children through `FUN_0060F9A0`, applies parent subclassing, then marks shell status capability byte `+0xD5 = 1` for a set that includes `0x6B`. Evidence: fresh decompile of `0x00622820`, branch `iVar1 == 0x6B`.

Implementation implication: the modal uses its own child `0x695`; parent `0x102` hover/status logic is not active because the setup dialog is hidden.

### 3.2 Common `WM_MOUSEMOVE` status write chain

Active in YR: Yes. Fresh assembly spot-check of the common subclass path confirms the settled chain:

1. `0x00611CBA` tests `WM_MOUSEMOVE (0x200)`.
2. `0x00611CD6..0x00611CE4` obtains parent child `0x695`; if absent, no status write happens.
3. `0x00611D1A..0x00611D20` sends `0x4E8` to the hovered child with packed cursor coordinates.
4. `0x00611D3C..0x00611D46` sends `0x4E9` to the parent with child/index context.
5. If empty, a second parent `0x4E9` request with index `-1` is attempted.
6. `0x00611E28..0x00611E31` calls `FUN_006040B0(parent, hovered_child)`.
7. `0x00611E3A..0x00611E4D` localizes a returned key through the string table; if no key exists, `0x00887734` blank is used.
8. `0x00611E85..0x00611E8B` sends `0x4B2` to the status child.

Implementation implication: status text is a hover-side update, not click-side state.

### 3.3 Exact `0x6B` fallback keys

Active in YR: Yes. Fresh decompile of `FUN_006040B0` confirms the `iVar4 == 0x6B` branch:

| Hovered control | Native key | Evidence |
|---:|---|---|
| `0x6EB` mode/game type list | `STT:ScenarioListGameType` | `FUN_006040B0`, string `0x008348AC` |
| `0x553` map list | `STT:ScenarioListMaps` | `FUN_006040B0`, string `0x00834894` |
| `0x468` map thumbnail | `STT:ScenarioMapThumbnail` | `FUN_006040B0`, string `0x00834878` |
| `0x6C5` Use Map button | `STT:ScenarioButtonUseMap` | `FUN_006040B0`, string `0x0083485C` |
| `0x583` Create Random Map button | `STT:ScenarioButtonRandom` | `FUN_006040B0`, string `0x00834840` |
| `0x5C0` Cancel button | `STT:ScenarioButtonCancel` | `FUN_006040B0`, string `0x00834824` |
| `0x695`, `-1` statics, unknown children | no key, blank when status path runs | no case in `0x6B` branch; empty fallback at `0x00611E50` |

Fresh string search found exactly the six `STT:Scenario*` strings above.

### 3.4 Mode-list row override

Active in YR: Yes. The `0x6B` callback handles parent message `0x4E9` at `0x005E6E44..0x005E6E97`. It clears the output, checks hovered control id, and only special-cases `0x6EB`. If the child is `0x6EB` and index is not `-1`, it sends `LB_GETITEMDATA (0x199)` to the list, treats the item data as a mode record, and copies text from record `+0x24`. Evidence: fresh assembly `0x005E6E5F CMP EDI,0x6EB`, `0x005E6E6A CMP ESI,-1`, `0x005E6E76 PUSH 0x199`, `0x005E6E87 LEA ECX,[EAX+0x24]`, `0x005E6E92 CALL 0x007B6880`.

Current Rust has `SkirmishGameMode::tooltip_key`, so it has a plausible implementation source for this row-specific text. This is an inference from Rust field naming plus the native `+0x24` role; the exact Rust field parity should be checked against the MPModes construction report before claiming complete binary parity.

### 3.5 Blank and sticky fallback

Active in YR: Yes for child/unmapped clearing. If the status path runs over `0x695`, `-1` statics/headings, or unknown children, `FUN_006040B0` returns null and the subclass sends the empty string. Evidence: no mapped cases plus `0x00611E50` blank fallback.

Active in YR: Conditional/uncertain for true parent-background hover. Static evidence still does not prove that moving from a mapped child into empty parent background writes blank. The dialog proc default path around `0x005E7038` returns zero, and this recheck found no direct `0x4B2` parent-background write. Runtime capture is needed to decide sticky-vs-clear for empty dialog chrome.

Implementation implication: clear on unmapped modal child/status strip is verified. Clearing on all empty dialog background is a policy choice until runtime capture proves native behavior.

## 4. INI Keys

No `rules.ini`, `rulesmd.ini`, `art.ini`, or `artmd.ini` keys control this hover mapping. The relevant source is dialog/control ids plus CSF/string table keys. Active in YR: Yes; no TS legacy INI gate was found.

## 5. Integration Points

| Point | Role | Evidence | Active in YR |
|---|---|---|---|
| `FUN_006ACEE0` | Parent setup command handler; enters chooser | `0x006AD947` | Yes |
| `FUN_005E68A0` | Creates/pumps dialog `0x6B` | fresh decompile; `0x005E68B7..0x005E68D5` | Yes |
| `FUN_00622820` | Common shell setup and status/subclass capability | fresh decompile; `iVar1 == 0x6B` in `+0xD5` branch | Yes |
| Common subclass thunk | Writes hover text to child `0x695` on `WM_MOUSEMOVE` | `0x00611CBA..0x00611E8B` | Yes |
| Callback `0x005E6920` | Parent `0x4E9` mode-row override | `0x005E6E44..0x005E6E97` | Yes |
| `FUN_006040B0` | Generic dialog/control to status key mapping | fresh decompile and string search | Yes |

## 6. Current Rust Implementation Status

Current Rust scan:

| Rust surface | Current status | Evidence |
|---|---|---|
| Shared status state | Exists; `status_help_text` defaults blank and setter/clear helpers exist | `src/ui/skirmish_shell/state.rs:832`, `state.rs:907` |
| Parent `0x102` resolver | Exists; `hovered_shell_control` blocks parent targets when modal/validation owns input | `src/ui/skirmish_shell/state.rs:1321`, `state.rs:1380` |
| Parent mouse move | Resolves parent hover status when no modal is open | `src/app.rs:991`, `src/app.rs:1166` |
| Modal mouse move | Missing behavior: if `choose_map_modal` or validation modal is open, it clears shared status text and returns | `src/app.rs:1166` |
| Modal layout | Has correct `0x6B` status strip and control rects needed for hit testing | `src/ui/skirmish_shell/layout.rs:653`, `layout.rs:810`, `layout.rs:834` |
| Modal text render | Draws headings/buttons/list rows, but does not draw `status_help_text` into modal `layout.status_help` | `src/app_skirmish_shell_render.rs:2181` |
| Parent stale status suppression | Explicitly returns `None` for parent status while modal is rendered | `src/app_skirmish_shell_render.rs:2177` |
| `STT:Scenario*` keys | Absent from Rust source | `rg "STT:Scenario"` returned no matches |
| Mode row data | Rust has `SkirmishGameMode::tooltip_key`, but no modal hover resolver consumes it | `src/skirmish_modes.rs:20` |

Important correction to prior handoff: Rust no longer merely lacks a status strip. It has status state and both parent/modal rects. The missing work is modal-specific hover hit testing, key selection, mode-row tooltip selection, and modal status rendering.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Target question/non-goals/stop conditions | verified | Section 0 | none |
| Prior report reconciliation | verified | `SKIRMISH_CHOOSE_MAP_0X6B_STATUS_HOVER_MAPPING_GHIDRA_REPORT.md` | none |
| Active `0x6B` modal path | verified | `0x006AD947`, `0x005E68A0`, `0x005E68B7..0x005E68D5` | none |
| `FUN_00622820` includes `0x6B` in shell status setup | verified | fresh decompile `0x00622820` | none |
| Common subclass `WM_MOUSEMOVE` status write | verified | `0x00611CBA..0x00611E8B` | none for mapped child hover |
| `0x6B` `STT:Scenario*` fallback mapping | verified | `FUN_006040B0`, string search | none |
| Mode-list row override | verified | `0x005E6E44..0x005E6E97` | exact Rust `tooltip_key` parity with native record `+0x24` should be verified via MPModes report when implementing |
| Child/unmapped/status-strip blanking | verified | no `0x695`/`-1` branch; empty fallback `0x00887734` | none |
| True parent-background blank/sticky | touched-not-exhausted | no static `0x4B2` parent-background write found | runtime capture |
| Current Rust parent status implementation | verified | `src/app.rs`, `src/ui/skirmish_shell/state.rs` | none |
| Current Rust modal status implementation | verified gap | `src/app.rs:1166`; no `STT:Scenario*` source matches | implement |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-01 - Is Choose Map dialog 0x6B active in standard offline YR? -> Yes, the Skirmish `0x5AA` path calls `FUN_005E68A0`.` (evidence: `0x006AD947`)
- `[RESOLVED] OQ-02 - Does `FUN_005E68A0` create dialog id 0x6B and install shell setup? -> Yes, it loads `0x6B`, passes callback `0x005E6920`, stores the HWND, and calls `FUN_00622820`.` (evidence: `0x005E68B7..0x005E68D5`)
- `[RESOLVED] OQ-03 - Is 0x6B included in common shell status/subclass handling? -> Yes, `FUN_00622820` includes `0x6B` in the `+0xD5` status-capability branch.` (evidence: fresh decompile `0x00622820`)
- `[RESOLVED] OQ-04 - What event updates modal status text? -> Common subclass `WM_MOUSEMOVE`, which writes through `0x4B2` to child `0x695`.` (evidence: `0x00611CBA..0x00611E8B`)
- `[RESOLVED] OQ-05 - What are the exact generic fallback keys? -> `0x6EB/0x553/0x468/0x6C5/0x583/0x5C0` map to the six `STT:Scenario*` keys listed in Section 3.3.` (evidence: `FUN_006040B0`, string search)
- `[RESOLVED] OQ-06 - Does mode-list hover have a row-specific override? -> Yes, valid `0x6EB` row/index can use mode record `+0x24` before generic fallback.` (evidence: `0x005E6E44..0x005E6E97`)
- `[RESOLVED] OQ-07 - Does 0x695 have a self-tooltip? -> No; no `0x695` case exists and fallback is blank when the status path runs.` (evidence: `FUN_006040B0`, `0x00611E50`)
- `[RESOLVED] OQ-08 - Are INI keys involved? -> No scoped INI reader controls this path; keys are CSF/string-table keys.` (evidence: `FUN_006040B0`, string search)
- `[RESOLVED] OQ-09 - Is this TS legacy? -> No, the path is live in standard YR Skirmish modal UI with no TS-only gate found.` (evidence: `0x006ACEE0`, `0x005E68A0`, `0x00622820`)
- `[RESOLVED] OQ-10 - Does current Rust still lack the parent status strip entirely? -> No; current Rust has parent and modal `status_help` rects plus shared state.` (evidence: `src/ui/skirmish_shell/layout.rs:653`, `state.rs:832`)
- `[RESOLVED] OQ-11 - Does current Rust implement modal hover mapping? -> No; modal mouse move clears status and returns, and no `STT:Scenario*` keys exist in source.` (evidence: `src/app.rs:1166`; negative `rg "STT:Scenario"`)
- `[RESOLVED] OQ-12 - Is parent stale status text reused while modal is drawn? -> No in current Rust rendering; test helper suppresses it, and mouse move clears it.` (evidence: `src/app_skirmish_shell_render.rs:2177`, `src/app.rs:1166`)
- `[DEFERRED] OQ-13 - Does true parent-background hover clear or leave the last status text?` (category: `needs-runtime-debugger`; reason: static code proves child/unmapped clearing but not parent-background clearing; next-step-if-pursued: native cursor capture from mapped control to empty modal chrome)
- `[DEFERRED] OQ-14 - Is Rust `SkirmishGameMode::tooltip_key` exactly the native record `+0x24` string for all rows?` (category: `requires-different-system-context`; reason: this slot verifies routing, not MPModes payload construction; next-step-if-pursued: check MPModes row-construction report before implementing item-specific mode hover)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| While `0x6B` is open, status updates target the modal's child `0x695`, not parent setup `0x102`. | `0x006AD947`, `0x005E68A0`, `0x00611CD6..0x00611E8B` | partial: parent targets are blocked and stale parent status is suppressed, but modal `0x695` text is not rendered/resolved | `src/app.rs`, `src/app_skirmish_shell_render.rs`, `src/ui/skirmish_shell/layout.rs` | Add a modal-specific hover resolver and draw non-empty `status_help_text` into `ChooseMapModalLayout.status_help`. | Open Choose Map, hover Use Map: modal status strip displays Use Map help while parent controls remain inactive. | Do not route modal hover through parent `0x102` `STT:Skirmish*` mapping. |
| Generic modal hover keys are exactly `STT:ScenarioListGameType`, `STT:ScenarioListMaps`, `STT:ScenarioMapThumbnail`, `STT:ScenarioButtonUseMap`, `STT:ScenarioButtonRandom`, `STT:ScenarioButtonCancel`. | `FUN_006040B0`, strings `0x00834824..0x008348AC` | missing: no `STT:Scenario*` source matches | `src/ui/skirmish_shell/state.rs` or new modal helper; `src/app.rs` mouse move | Hit-test modal listboxes, preview, and buttons to those exact CSF keys and localize through existing CSF path. | Hover map list -> maps help; hover preview -> thumbnail help; hover Cancel -> cancel help. | Do not use button captions or English literals as status text. |
| Valid mode-list row hover can use item-specific mode text before generic `STT:ScenarioListGameType`. | `0x005E6E44..0x005E6E97` | missing/unchecked; Rust has `SkirmishGameMode::tooltip_key` but resolver does not consume it | `ChooseMapModalState`, `src/skirmish_modes.rs`, modal hover resolver | For `0x6EB` row hit, prefer the row's verified tooltip text if available; otherwise consciously fall back to generic and mark the gap. | Hover Battle mode row: row-specific mode help appears if `tooltip_key` is confirmed/wired; otherwise a focused test documents temporary generic fallback. | Do not claim generic `STT:ScenarioListGameType` is always native for row hover. |
| Status strip self-hover and unmapped/id `-1` modal children clear to blank when the status path runs. | no `0x695`/`-1` case in `FUN_006040B0`; empty fallback `0x00611E50` | partial: current modal move clears everything, but only by skipping all modal mapping | modal hover resolver | Return blank for `layout.status_help`, headings/statics, unknown children, and unmapped modal child areas where child status path applies. | Hover Use Map then the status strip itself: text clears. | Do not add a self-tooltip to `0x695`. |
| True empty parent-background sticky-vs-clear remains unproven statically. | `0x005E7038` default path; no direct parent `0x4B2` write found | current Rust clears while modal open on any move | `src/app.rs` modal hover policy | Either retain clear-on-empty as a documented simplification or capture native runtime and match it. | Runtime capture decides: mapped button -> empty chrome either clears or remains sticky; Rust follows evidence. | Do not mark "background definitely clears" as verified until runtime evidence exists. |

Proposed Rust test names:

- `choose_map_modal_hover_uses_scenario_status_keys`
- `choose_map_modal_hover_draws_status_in_modal_strip_not_parent_strip`
- `choose_map_modal_hover_mode_row_prefers_mode_tooltip_key`
- `choose_map_modal_hover_unmapped_child_clears_status_help`
- `choose_map_modal_hover_background_sticky_behavior_matches_runtime_capture`

## 10. Negative Facts / Do Not Do

- Do not reuse parent `0x102` `STT:Skirmish*` mappings while Choose Map `0x6B` is open. Active in YR: Yes; `FUN_006040B0` has a separate `0x6B` branch.
- Do not render a permanent status label such as `GUI:Blank`, "Status", current map, or dialog title. Active in YR: Yes; native fallback is blank unless hover supplies text.
- Do not give `0x695` a self-tooltip. Active in YR: Yes; no `0x695` case exists.
- Do not implement status as a click-side effect. Active in YR: Yes; update path is `WM_MOUSEMOVE`.
- Do not claim all mode-list hover text is generic. Active in YR: Conditional; valid rows can use record `+0x24`.
- Do not claim true empty modal background clearing is statically verified. Active in YR: Conditional/uncertain pending runtime capture.

## 11. Stale Docs / Follow-up Docs

- Replace any wording that says "Rust lacks `0x695` status/help layout/state" with: "STALE as of 2026-05-23 current Rust. Rust has parent and Choose Map `status_help` rects plus shared `status_help_text`; remaining gap is modal-specific `0x6B` hover resolution/rendering for `STT:Scenario*` and mode-row item text."
- Replace any wording that says "modal status should reuse parent `0x102` hover resolver" with: "Wrong for `0x6B`. Native modal status writes to the modal's child `0x695` through the common subclass path and uses the `0x6B` `STT:Scenario*` branch."
- Keep prior wording that true parent-background blanking is partial/runtime-only; this recheck did not upgrade it to verified.

## 12. Remaining Uncertainty

- True blank parent-background hover sticky-vs-clear needs runtime capture.
- Exact parity between current Rust `SkirmishGameMode::tooltip_key` and native mode record `+0x24` should be checked against the MPModes row-construction evidence before implementation claims mode-row hover complete.
- This report did not inspect localized CSF contents, only key routing.

## Sources

- Fresh Ghidra decompile: `0x005E68A0`, `0x00622820`, `0x006040B0`.
- Fresh Ghidra assembly/context: `0x006AD947`, `0x005E68B7..0x005E68D5`, `0x00611CBA`, `0x00611D17`, `0x00611D39`, `0x00611E28`, `0x00611E75`, `0x005E6E44`, `0x005E6E5F`, `0x005E6E92`.
- Fresh string search: `STT:ScenarioButtonCancel @ 0x00834824`, `STT:ScenarioButtonRandom @ 0x00834840`, `STT:ScenarioButtonUseMap @ 0x0083485C`, `STT:ScenarioMapThumbnail @ 0x00834878`, `STT:ScenarioListMaps @ 0x00834894`, `STT:ScenarioListGameType @ 0x008348AC`.
- Prior report: `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_CHOOSE_MAP_0X6B_STATUS_HOVER_MAPPING_GHIDRA_REPORT.md`.
- Current Rust scan: `C:/Users/enok/Documents/ra2-rust-game/src/app.rs`, `src/app_skirmish_shell_render.rs`, `src/ui/skirmish_shell/layout.rs`, `src/ui/skirmish_shell/state.rs`, `src/skirmish_modes.rs`.
