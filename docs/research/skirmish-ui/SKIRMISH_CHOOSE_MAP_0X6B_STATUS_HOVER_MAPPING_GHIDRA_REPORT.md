# Skirmish Choose Map 0x6B Status Hover Mapping - Ghidra Research Report

**Address(es):** `0x005E68A0`, `0x005E6920..0x005E7041`, `0x0060F9A0`, `0x00610CA0` thunk slice `0x00611CBA..0x00611E8B`, `0x006040B0`, `0x00622B50`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** Choose Map dialog `0x6B` status/help child `0x695`: visibility/default blank behavior, live hover update path, `0x6B` control-id to `STT:Scenario*` fallback mapping, 0x4E9 mode-list override, blank/status fallback behavior, and current Rust handoff.  
**Non-Scope:** Parent Skirmish setup `0x102` tooltip mapping except for comparison, modal geometry/button behavior already settled by prior reports, random map generation, keyboard/default-button behavior, and runtime screenshot capture.  
**Confidence:** High for `0x6B` creation/subclass installation, status child existence, mapped `STT:Scenario*` keys, status-strip self-hover clearing, and Rust missing hover resolver. Medium for blank parent-background clearing because the read-only code proves unmapped child clearing but not a parent-background mousemove clear.  
**Active in YR:** Yes for standard offline Yuri's Revenge Choose Map dialog reached from Skirmish `Choose Map`.

## 0. Working Notes Gate

- Target question: What exact native status/help behavior should Rust reproduce for Choose Map dialog `0x6B`, especially child `0x695` and hovered-control mapping?
- Non-goals: Do not re-investigate parent `0x102` mappings except for comparison, modal button geometry/sounds, list population, preview refresh, random map generation, or keyboard/default-button dismissal.
- Evidence needed to mark COMPLETE: active `0x6B` creation path, owner-draw/subclass install path that reaches status updates, `0x695` child visibility/default blank evidence, `0x6B` `FUN_006040B0` mapping with string addresses, fallback/blank behavior, current Rust scan, and YR liveness check.
- Stop conditions: Stop when all scoped open questions are resolved or explicitly deferred, the final Ghidra pass adds no new scoped child questions, and the handoff names exact Rust surfaces and acceptance scenarios.

## 1. Overview

Choose Map dialog `0x6B` has a real status/help static `0x695`. Native status text is not a permanent label: it is normally blank, then rewritten during hover processing by common owner-draw/subclass plumbing. For `0x6B`, `FUN_006040B0` maps listboxes, preview, and modal buttons to six `STT:Scenario*` keys; the mode list `0x6EB` has one earlier parent `0x4E9` override that can show item-specific mode text when a valid mode-list row index is supplied.

Active in YR: Yes. `FUN_006ACEE0 @ 0x006AD947` calls `FUN_005E68A0` from the standard Skirmish `Choose Map` button path. `FUN_005E68A0 @ 0x005E68B7..0x005E68D5` creates dialog id `0x6B`, stores `DAT_00AC0D40`, and calls `FUN_00622820`, which installs owner-draw/subclass handling on the dialog and children.

## 2. Class Layout / Key Values

| Value / field | Meaning | Evidence | Active in YR |
|---|---|---|---|
| Dialog id `0x6B` | Choose Map modal dialog | `0x005E68B7`/`0x005E68C9` load `0x6B`; `FUN_006040B0` has `iVar4 == 0x6B` branch | Yes |
| Dialog proc boundary | Ghidra lacks function boundary at `0x005E6920`; inspected read-only as assembly | `decompile_function 0x005E6920` fails in prior gap audit; assembly `0x005E6920..0x005E7041` | Yes |
| Child `0x695` | Status/help static in `0x6B` | resource/layout reports; subclass hover path calls `GetDlgItem(parent,0x695)` at `0x00611CD6..0x00611CE4` | Yes |
| Message `0x200` | `WM_MOUSEMOVE`; status update trigger in common subclass thunk | `0x00611CBA CMP ESI,0x200`, then status lookup/update path | Yes |
| Message `0x4E8` | Ask hovered child/control for status context | `0x00611D17..0x00611D20` sends `0x4E8` to hovered/subclassed hwnd | Yes |
| Message `0x4E9` | Ask parent dialog for item-specific status text | `0x00611D39..0x00611D46` and `0x00611DC8..0x00611DD9`; `0x005E6E44..0x005E6E97` handles `0x6B` parent case | Yes |
| Message `0x4B2` | Wide dynamic text write to status static | `0x00611E75..0x00611E8B` sends `0x4B2` to `0x695`; static text thunk report verifies copy | Yes |
| Empty wide string `0x00887734` | Blank fallback when no source text exists | `0x00611E50` pushes empty; `FUN_00622B50` also uses same fallback | Yes |

## 3. Core Logic

### 3.1 Active 0x6B creation and subclass installation

Active in YR: Yes. `FUN_006ACEE0` is the standard offline Skirmish command handler. Its `0x5AA` branch hides the parent dialog and calls `FUN_005E68A0` (`0x006AD947`). `FUN_005E68A0` constructs the modal surface, loads the Customize Battle shell asset path, creates dialog resource `0x6B` with callback address `0x005E6920`, stores the HWND in `DAT_00AC0D40`, calls `FUN_00622820`, sends init `0x4A9`, shows the dialog, and enters the modal loop.

`FUN_00622820` is live for `0x6B`. It enumerates children through `FUN_0060F9A0`, calls `FUN_0060F9A0` on the parent, and then marks shell-status capability byte `record+0xD5 = 1` for dialog ids including `0x6B`. Evidence: decompile `FUN_00622820`, branch containing `iVar1 == 0x6B`.

### 3.2 Status update path for 0x6B is the common subclass WM_MOUSEMOVE path

Active in YR: Yes. For `0x6B`, the dialog proc at `0x005E6920` does not show a direct call to `FUN_00622B50`; its `WM_MOUSEMOVE`-sized default path falls through to zero return. The status update instead appears in the common subclass thunk installed by `FUN_0060F9A0`:

1. On message `0x200`, the thunk obtains the hovered control's parent and then `GetDlgItem(parent,0x695)` (`0x00611CBA..0x00611CE4`).
2. If no status child is found, it skips the status write (`0x00611CE8 -> 0x00611E9A`).
3. It sends `0x4E8` to the hovered/subclassed control with packed cursor coordinates (`0x00611CF7..0x00611D20`).
4. It sends `0x4E9` to the parent with the hovered hwnd plus the result/index from the child path (`0x00611D31..0x00611D46`).
5. If the resulting text is empty, it sends a second parent `0x4E9` request with index `-1` (`0x00611DB0..0x00611DD9`).
6. If still empty, it calls `FUN_006040B0(parent, hovered_hwnd)` (`0x00611E28..0x00611E31`).
7. If `FUN_006040B0` returns a key pointer, it loads the localized string via `StringTable__LoadString(...,0x7A5)` (`0x00611E3A..0x00611E4D`); otherwise it uses empty `0x00887734` (`0x00611E50`).
8. It sends the selected wide text to status child `0x695` via `SendMessageA(status,0x4B2,0,text)` (`0x00611E75..0x00611E8B`).

Comparison only: `FUN_00622B50` has a similar status update chain on `WM_NCHITTEST` and calls `FUN_006040B0` at `0x00622E21`, but that parent-proc path is not the proven `0x6B` modal hover path in this slice. The shared dispatcher `FUN_006040B0` is still the same final fallback.

### 3.3 Parent 0x4E9 override is narrow: mode list row text only

Active in YR: Yes. The `0x6B` callback's `0x4E9` branch starts at `0x005E6E44`. It clears the outgoing text (`FUN_007B6880(0)`), reads the hovered control id, and only handles child `0x6EB`.

If the hovered control id is `0x6EB` and the supplied row/index is not `-1`, it sends listbox message `0x199` to `0x6EB` with that index, treats the returned item data as a mode record pointer, reads text from `record+0x24`, and writes that into the outgoing status string (`0x005E6E5F..0x005E6E92`). If the id is not `0x6EB`, if the index is `-1`, or if item data is null, the parent override leaves the outgoing string empty and returns handled.

Rust-facing nuance: hovering a concrete mode-list row may display mode-specific text before the generic `STT:ScenarioListGameType` fallback is considered. The map list `0x553`, preview `0x468`, and buttons have no equivalent `0x4E9` branch in `0x005E6920`; they use the generic `STT:Scenario*` fallback unless their own child `0x4E8` supplies something, which this slice did not find for those controls.

### 3.4 Fallback STT:Scenario mapping

Active in YR: Yes. `FUN_006040B0(parent, hovered_child)` resolves the dialog id from the parent owner-draw record, gets the hovered child id with `GetDlgCtrlID`, rejects child ids `0` and `-1`, and branches on dialog id. The `iVar4 == 0x6B` branch maps exactly:

| Hovered control id | String pointer address | Key | Active in YR |
|---:|---:|---|---|
| `0x6EB` | `0x008348AC` | `STT:ScenarioListGameType` | Yes; generic fallback after any mode-row override |
| `0x553` | `0x00834894` | `STT:ScenarioListMaps` | Yes |
| `0x468` | `0x00834878` | `STT:ScenarioMapThumbnail` | Yes |
| `0x6C5` | `0x0083485C` | `STT:ScenarioButtonUseMap` | Yes |
| `0x583` | `0x00834840` | `STT:ScenarioButtonRandom` | Yes |
| `0x5C0` | `0x00834824` | `STT:ScenarioButtonCancel` | Yes |
| `0x695`, id `-1`, parent/background, any other id | none | empty fallback when the status path runs | Yes/Conditional; see blank behavior below |

Assembly proof: string xrefs in `FUN_006040B0` are at `0x006051E6`, `0x006051F6`, `0x00605206`, `0x00605216`, `0x00605226`, and `0x0060523A`. Context shows the compare chain `CMP ESI,0x6B`, then child-id compares `0x6EB`, `0x553`, `0x468`, `0x6C5`, `0x583`, `0x5C0`, with fallback jump to null return.

### 3.5 Visibility, default blank, and blank/status fallback

Active in YR: Yes. Resource/layout reports verify `0x695` exists in dialog `0x6B` at local `(2,355,303,12)` and current Rust already models this rect. The text source is blank by default: no permanent label is assigned by this hover path, and both common status paths use empty `0x00887734` when no source text exists.

Status-strip self-hover: Active in YR: Yes. `0x695` is a child of the dialog, so the subclass status path can find `GetDlgItem(parent,0x695)`. `FUN_006040B0` has no `0x695` case in the `0x6B` branch, so the path sends empty text to `0x695`.

Unmapped child/static hover: Active in YR: Yes. For id `-1` statics or any child id not in the `0x6B` mapping, `FUN_006040B0` returns null and the subclass path sends the empty string.

Blank parent-background hover: Active in YR: Conditional/uncertain. The read-only evidence proves the subclass path clears when the mousemove is delivered to a child whose parent has `0x695`. For true parent-background mousemove, the dialog proc default path at `0x005E7038` returns zero and no direct `0x4B2` write was found. If runtime sends the mousemove to the parent rather than an unmapped child, native may leave the previous status text until another child/status hover occurs. Do not rely on a "parent blank always clears" claim without runtime capture.

## 4. INI Keys

No `rules.ini`, `rulesmd.ini`, `art.ini`, or `artmd.ini` keys control this status/help mapping. The mapped keys are CSF/string-table `STT:*` names and shell resource/control ids. Active in YR: Yes; no TS-legacy INI gate was found in this UI path.

## 5. Integration Points

| Point | Role | Evidence | Active in YR |
|---|---|---|---|
| `FUN_006ACEE0` | Standard Skirmish command handler; calls Choose Map modal | `0x006AD947` call to `FUN_005E68A0` | Yes |
| `FUN_005E68A0` | Creates dialog `0x6B`, calls shell setup, shows modal | decompile; assembly `0x005E68B7..0x005E68F8` | Yes |
| `FUN_00622820` | Installs subclass/owner-draw handlers and marks `0x6B` shell status capability | decompile branch includes `iVar1 == 0x6B` | Yes |
| Common subclass thunk `0x00610CA0` | On `WM_MOUSEMOVE`, writes hover text to parent child `0x695` | read-only assembly `0x00611CBA..0x00611E8B` | Yes |
| Callback `0x005E6920` | Handles `0x4E9` mode-list item override | read-only assembly `0x005E6E44..0x005E6E97` | Yes |
| `FUN_006040B0` | Generic dialog/control id to status key dispatcher | decompile and string xrefs | Yes |
| `0x00610CA0` / static text reports | `0x4B2` text copy into owner-draw static record | prior static-thunk report | Yes |

## 6. Current Rust Implementation Status

Current Rust has the storage and render slot but not the native 0x6B hover resolver.

- `src/ui/skirmish_shell/state.rs`: `SkirmishShellState::status_help_text` exists and defaults blank.
- `src/app_skirmish_shell_render.rs`: modal rendering uses `state.skirmish_shell_state.status_help_text` for `layout.status_help` and suppresses text when empty.
- `src/app.rs`: `handle_skirmish_shell_mouse_move` returns immediately while `choose_map_modal` is open; no modal hover hit-test updates `status_help_text`.
- `src/`: no occurrences of `STT:ScenarioListGameType`, `STT:ScenarioListMaps`, `STT:ScenarioMapThumbnail`, `STT:ScenarioButtonUseMap`, `STT:ScenarioButtonRandom`, or `STT:ScenarioButtonCancel` were found.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Standard Skirmish Choose Map reaches `0x6B` | verified | `0x006AD947`, `FUN_005E68A0` | none |
| `0x6B` child `0x695` resource/layout presence | verified-by-prior-plus-current scan | visual layout/rect reports; current `layout.rs` | runtime pixel capture optional |
| `FUN_00622820` subclass setup for `0x6B` | verified | decompile `FUN_00622820`; `iVar1 == 0x6B` branch | none |
| Common subclass `WM_MOUSEMOVE` status write | verified | assembly `0x00611CBA..0x00611E8B` | exact function boundary still absent by read-only constraint |
| `0x6B` parent `0x4E9` mode-list override | verified | assembly `0x005E6E44..0x005E6E97` | exact CSF contents of mode record text not decoded here |
| `FUN_006040B0` `0x6B` fallback keys | verified | decompile; xrefs `0x006051E6..0x0060523A` | none |
| Status strip self-hover blank | verified | no `0x695` case in `FUN_006040B0`; empty fallback in thunk | none |
| Blank true parent-background clearing | touched-not-exhausted | `0x005E7038` default return; no direct status write found | runtime capture to distinguish sticky vs clear |
| Current Rust status/render field | verified | source scan | implementation of hover resolver remains |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-01 - Is Choose Map dialog 0x6B active in standard offline YR? -> Yes, `FUN_006ACEE0` calls `FUN_005E68A0` from the `0x5AA` command path.` (evidence: `0x006AD947`)
- `[RESOLVED] OQ-02 - Does `FUN_005E68A0` create resource id 0x6B? -> Yes, assembly loads `EDX=0x6B` before/after `0x00775700`.` (evidence: `0x005E68B7..0x005E68C9`)
- `[RESOLVED] OQ-03 - Is child 0x695 present in 0x6B? -> Yes by resource/layout reports and live status lookup.` (evidence: `0x00611CD6..0x00611CE4`; modal rect reports)
- `[RESOLVED] OQ-04 - Is `0x695` default/permanent text non-empty? -> No verified permanent text; empty `0x00887734` is used when no source exists.` (evidence: `0x00611E50`; prior rect/resource reports)
- `[RESOLVED] OQ-05 - What live event updates 0x6B status text? -> Common subclass `WM_MOUSEMOVE` (`0x200`) path, not a tick.` (evidence: `0x00611CBA..0x00611E8B`)
- `[RESOLVED] OQ-06 - Does the 0x6B dialog proc call the 0x102-style common parent handler directly? -> No direct `FUN_00622B50` call was found in the read-only xrefs/assembly for `0x005E6920`; subclass path supplies the proven 0x6B hover update.` (evidence: xrefs to `0x00622B50`; assembly `0x005E6920..0x005E7041`)
- `[RESOLVED] OQ-07 - Does parent 0x4E9 override any generic key? -> Yes, only mode-list `0x6EB` with a valid row/index and non-null item data can write mode record text from `record+0x24`.` (evidence: `0x005E6E44..0x005E6E97`)
- `[RESOLVED] OQ-08 - What does hovering mode list 0x6EB display? -> A valid row may show item-specific mode text; otherwise fallback is `STT:ScenarioListGameType`.` (evidence: `0x005E6E5F..0x005E6E92`; `0x006051E6`)
- `[RESOLVED] OQ-09 - What does hovering map list 0x553 display? -> Generic fallback `STT:ScenarioListMaps`.` (evidence: `0x006051EE..0x006051F6`)
- `[RESOLVED] OQ-10 - What does hovering preview 0x468 display? -> Generic fallback `STT:ScenarioMapThumbnail`.` (evidence: `0x006051FE..0x00605206`)
- `[RESOLVED] OQ-11 - What do modal buttons display? -> Use Map `STT:ScenarioButtonUseMap`, Create Random `STT:ScenarioButtonRandom`, Cancel `STT:ScenarioButtonCancel`.` (evidence: `0x0060520E..0x0060523A`)
- `[RESOLVED] OQ-12 - Does status strip 0x695 have a self-tooltip? -> No; it falls through to empty when the status path runs.` (evidence: full `0x6B` branch in `FUN_006040B0`)
- `[RESOLVED] OQ-13 - Are INI keys involved? -> No scoped INI reader or key affects these mappings.` (evidence: dispatcher/string-table path; INI scan negative)
- `[RESOLVED] OQ-14 - Is this TS legacy? -> No, this is live shell UI on the standard YR Skirmish modal path with no TS-only gate found.` (evidence: `FUN_006ACEE0`, `FUN_005E68A0`, `FUN_00622820`)
- `[RESOLVED] OQ-15 - Does current Rust implement the 0x6B hover resolver? -> No; modal mouse move returns early and no `STT:Scenario*` mapping exists in source.` (evidence: `src/app.rs`, `src/app_skirmish_shell_render.rs`, `src/ui/skirmish_shell/state.rs` scan)
- `[DEFERRED] OQ-16 - Does true parent-background hover clear or leave the last status text?` (category: needs-runtime-debugger; reason: assembly proves unmapped child/status clearing but the modal proc default path has no status write for parent `WM_MOUSEMOVE`; next-step-if-pursued: capture native cursor movement from a mapped button into empty dialog background)
- `[DEFERRED] OQ-17 - What exact localized contents are stored at mode record `+0x24` for every mode row?` (category: out-of-scope; reason: this target is status routing/mapping, not MPModes string population; next-step-if-pursued: consume MPModes reports or decode mode record construction)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| `0x6B` status text is blank by default and only drawn when the current hover resolver produces text. | `0x00611E50`, `0x00611E75..0x00611E8B`; resource/layout reports | mostly present | `src/ui/skirmish_shell/state.rs`, `src/app_skirmish_shell_render.rs` | Keep `status_help_text` default empty and render no visible fallback label when empty. | Open Choose Map with no hover-derived status: strip has no hardcoded text. | Do not render `GUI:Blank`, "Status", current map name, or dialog title as permanent status text. |
| Modal hover updates should run while `choose_map_modal` is open. | `0x00611CBA..0x00611E8B` | missing: `handle_skirmish_shell_mouse_move` returns early when modal is open | `src/app.rs`; likely a new modal hover resolver in `ui/skirmish_shell` | On mouse move over modal children, set/clear `status_help_text` from verified control identity. | Move from Use Map to Cancel and the status text changes from Use Map help to Cancel help without clicking. | Do not leave parent `0x102` hover resolver active under the modal; parent controls are hidden/suppressed. |
| Fallback `STT:Scenario*` mapping is exact for `0x6EB`, `0x553`, `0x468`, `0x6C5`, `0x583`, `0x5C0`. | `FUN_006040B0`; xrefs `0x006051E6..0x0060523A`; string search addresses `0x00834824..0x008348AC` | missing | `src/ui/skirmish_shell/layout.rs` hit testing, `src/app.rs` mouse move, localization/CSF label path | Map hovered modal listboxes/preview/buttons to these exact keys, then localize through the same shell text path used for other CSF labels. | Hover map list -> `STT:ScenarioListMaps`; preview -> `STT:ScenarioMapThumbnail`; Create Random -> `STT:ScenarioButtonRandom`. | Do not use visible button captions or English literals as status text. |
| Mode-list row hover can have item-specific text before generic `STT:ScenarioListGameType`. | `0x005E6E44..0x005E6E97` | missing/unchecked | `ChooseMapModalState` mode rows; status resolver | If Rust has the native mode description/string equivalent, use it for valid `0x6EB` row hover; otherwise fall back consciously to `STT:ScenarioListGameType` and mark the item-specific text gap. | Hover a populated mode row: either native mode help text appears, or a test documents the temporary generic fallback as a known gap. | Do not claim generic `STT:ScenarioListGameType` is always native for mode-row hover. |
| Status strip self-hover and unmapped child/static hover clear to blank when the status path runs. | no `0x695`/`-1` cases in `FUN_006040B0`; empty fallback `0x00611E50` | missing | modal hover hit test | Hover `0x695`, headings/id `-1` statics, or unknown modal child and clear `status_help_text`. | Hover status strip after Use Map: status becomes blank. | Do not add a self-tooltip for `0x695`. |
| Blank parent-background clearing is not fully proven from static code. | `0x005E7038` default return; no direct parent `0x4B2` write found | unchecked | modal hover resolver policy | Prefer a conservative clear-on-unmapped-dialog-area only if accepted as Rust simplification, or verify runtime first. | Runtime follow-up: mapped button -> empty dialog background either clears or stays sticky; Rust matches chosen evidence. | Do not record "blank dialog background definitely clears" as a verified native fact without runtime evidence. |

### Negative Facts / Do Not Do

- Do not use parent `0x102` `STT:Skirmish*` mappings while the `0x6B` modal is open. Active in YR: Yes. Evidence: `FUN_006040B0` has a separate `iVar4 == 0x6B` branch.
- Do not give `0x695` a self-tooltip. Active in YR: Yes. Evidence: no `0x695` case in the `0x6B` branch; empty fallback.
- Do not assume all mode-list hover text is the generic `STT:ScenarioListGameType`. Active in YR: Conditional. Evidence: `0x005E6E44..0x005E6E97` can supply mode-record text for valid rows.
- Do not implement status text as a click-side effect. Active in YR: Yes. Evidence: update path is `WM_MOUSEMOVE`/hover-side routing and `0x4B2`, independent of button activation.

## Sources

- Ghidra read-only decompile: `FUN_005E68A0`, `FUN_006ACEE0`, `FUN_00622820`, `FUN_006040B0`, `FUN_00622B50`, `FUN_00603F00`.
- Ghidra read-only assembly/context: `0x005E68B7..0x005E68F8`, `0x005E6920..0x005E7041`, `0x005E6E44..0x005E6E97`, `0x00611CBA..0x00611E8B`, `0x006051D9..0x0060523A`, `0x00622CCB..0x00622E83`.
- Ghidra string search: `STT:ScenarioButtonCancel @ 0x00834824`, `STT:ScenarioButtonRandom @ 0x00834840`, `STT:ScenarioButtonUseMap @ 0x0083485C`, `STT:ScenarioMapThumbnail @ 0x00834878`, `STT:ScenarioListMaps @ 0x00834894`, `STT:ScenarioListGameType @ 0x008348AC`.
- Prior docs referenced: `SKIRMISH_CHOOSE_MAP_0X6B_POST_IMPLEMENTATION_GAP_AUDIT_GHIDRA_REPORT.md`, `SKIRMISH_STATUS_CHILD_0X695_TEXT_SOURCE_GHIDRA_REPORT.md`, `SKIRMISH_CHOOSE_MAP_MODAL_0X6B_RECT_BOUNDARY_GHIDRA_REPORT.md`, `SKIRMISH_STATIC_TEXT_SUBCLASS_THUNK_00610CA0_GHIDRA_REPORT.md`, `SKIRMISH_SUBCLASS_THUNK_00610CA0_NON_TEXT_BEHAVIOR_GHIDRA_REPORT.md`.
- Rust read-only scan: `src/app.rs`, `src/app_skirmish_shell_render.rs`, `src/ui/skirmish_shell/layout.rs`, `src/ui/skirmish_shell/state.rs`, `src/ui/skirmish_shell/mod.rs`.
