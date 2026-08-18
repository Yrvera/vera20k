# Skirmish Create Random Map 0x583 Accept/Cancel State Machine - Ghidra Research Report

**Address(es):** `0x006ACEE0`, `0x005E68A0`, `LAB_005E6920`, `0x005E8590`, `0x00595BC0`, `0x00596300`, `0x005E70D0`, `0x005E7160`, `0x005E7BF0`, `0x0069ADF0`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** Standard offline YR Choose Map command `0x583` accept/cancel/failure state machine: parent modal result boundary, nested random-map setup result boundary, selection preservation, synthetic `RandMap.Sed` record selection, parent preview preservation vs random preview side effects, and active-YR liveness.
**Non-Scope:** Random-map dialog full visual layout, complete random-map setup control semantics, full `.SED` file layout, random terrain generation formulas, launch branch internals after `.SED` detection, online/WOL variants, and exact `RandMap.img` pixels.
**Confidence:** High for accept/cancel/failure state machine and selection/commit boundaries; Medium for exact user-visible impact of setup-cancel `RandMap.img` file overwrite because static evidence proves the write path but not a runtime screenshot.
**Active in YR:** Conditional. The path is live in standard offline YR Choose Map dialog `0x6B`; `0x583` behavior is conditional on the player pressing Create Random Map and, for commit, the nested random-map setup returning result `1`.

## Working Notes Gate

- Target question: Verify when standard YR Choose Map command `0x583` preserves the existing map selection/preview/state, when it selects the synthetic `RandMap.Sed` scenario record, which return values gate side effects, and whether this is live YR or TS legacy.
- Non-goals: Do not investigate terrain formulas, full random-map dialog controls/options, full `.SED` serialization, launch generation, modal paint, or ordinary Choose Map visual/listbox parity outside the `0x583` state transition.
- Evidence needed to mark COMPLETE: parent `0x5AA -> 0x6B` active path, chooser callback command split for `0x5C0`/`0x6C5`/`0x583`, nested `0x005E8590` result gate, random-map setup dialog accept/cancel result writes, post-success reselect/commit path, parent cancel/accept result handling, current Rust scan, and TS-legacy/liveness proof.
- Stop conditions: Stop once all state mutations visible to Rust selection, scenario records, preview wrapper/cache, and launch token are classified for success, nested cancel/failure, and parent cancel.
- Prior state row: Partial/high-confidence reports exist; proceed to gaps + verification only. This report narrows and corrects wording from the broader `0x583` and Choose Map reports rather than rediscovering generator internals.

## 1. Overview

`Create Random Map` is a nested transaction inside the live Choose Map modal. The Choose Map callback hides the chooser and calls `0x005E8590`; only when that helper returns an index other than `-1` does the callback rebuild/reselect the `RandMap.Sed` row and commit through the ordinary Use Map helper.

There are three distinct cancel/failure boundaries. Parent Choose Map Cancel returns result `2` to the Skirmish parent and restores the pre-modal selected mode/index. Nested random-map setup Cancel also writes result `2`, but `0x005E8590` converts that to `-1`, returns to the still-open chooser, and does not commit `RandMap.Sed`. Setup-window creation failure or early pump exit can also return non-`1` and follows the same no-commit path. A generated temporary preview may still be written to `RandMap.img` by setup teardown, so "no side effects" must be scoped to selection/sentinel/parent preview state, not every filesystem byte.

## 2. Key State / Return Values

| Item | Meaning | Evidence | Active in YR |
|---|---|---|---|
| parent command `0x5AA` | Opens Choose Map modal from Skirmish shell | `0x006AD947 CALL 0x005E68A0` | Yes |
| chooser command `0x5C0` | Choose Map Cancel; closes chooser with result `2` | `0x005E69E7 MOV EDX,0x2`; `0x005E69EC CALL 0x007757E0` | Conditional: user clicks Cancel |
| chooser command `0x6C5` | Choose Map Use Map; calls ordinary accept helper | `0x005E69C2 CMP EAX,0x6C5`; `0x005E6B63..0x005E6B67 CALL 0x005E7160` | Conditional: user clicks Use Map |
| chooser command `0x583` | Create Random Map; calls nested setup helper | `0x005E69D3 SUB EAX,0x583`; `0x005E6A11 CALL 0x005E8590` | Conditional: user clicks Create Random Map |
| nested setup result `1` | Accepted random-map setup | `0x005E85C6 CMP EAX,0x1`; fallthrough to save/upsert work | Conditional |
| nested setup result `2` | Random-map setup Cancel | `0x00596300`, command `0x5C0`: `*puVar5 = 2; return 1` | Conditional |
| nested non-`1` result | Converted to `-1` by `0x005E8590` | `0x005E85C6 CMP EAX,0x1`; `0x005E85CB OR EAX,0xFFFFFFFF`; `0x005E85CE RET` | Conditional |
| `DAT_00A8B250` | selected mode/category token | parent restore at `0x006AD95B` / `0x006ADB52`; commit in `0x005E7160` | Yes |
| `DAT_00A8B254` | selected scenario-record index | parent restore at `0x006AD961` / `0x006ADB4B`; commit in `0x005E7160` | Yes |
| `DAT_00A8B8CC` / `DAT_00A8B8D8` | scenario-record array/count | scanned and updated/appended by `0x005E8590` | Conditional |
| `DAT_00AC1154` | parent/chooser preview wrapper | replaced on accepted setup; not replaced on nested setup failure by `0x583` callback | Conditional |
| `DAT_00ABE154` | temporary random-map setup preview wrapper | teardown in `0x00595BC0` may write `RandMap.img` before freeing | Conditional |

## 3. Core Logic

### 3.1 Parent Choose Map transaction

Active in YR: Yes. The standard offline Skirmish parent command branch saves the old selected scenario index and mode token, hides the parent, calls `0x005E68A0`, and compares the modal result with `2`.

Evidence: decompile `0x006ACEE0`; assembly `0x006AD947 CALL 0x005E68A0`, `0x006AD94C CMP EAX,0x2`, `0x006AD94F JNZ 0x006ADA21`.

Parent result handling:

- Result `2`: parent cancel path restores old `DAT_00A8B250` then old `DAT_00A8B254`, reloads the old selected record, refreshes preview, and shows the parent.
- Any result other than `2`: parent accept path assumes the chooser committed selection through `0x005E7160`, rebuilds row/capacity state, shows the parent, then loads the selected record. If selected-record load fails, it restores old index/token and returns before normal label/preview refresh.

### 3.2 Choose Map Cancel `0x5C0` is parent cancel, not random setup cancel

Active in YR: Conditional. In the Choose Map callback, command low word `0x5C0` reaches `0x005E69E7`, writes modal result `2` through `0x007757E0`, and returns. The Skirmish parent observes that result at `0x006AD94C` and restores the pre-modal selection.

Evidence: assembly `0x005E69DA SUB EAX,0x3D`, `0x005E69DD JNZ`, `0x005E69E7 MOV EDX,0x2`, `0x005E69EC CALL 0x007757E0`; parent restore assembly `0x006AD95B`, `0x006AD961`.

### 3.3 Use Map `0x6C5` commits through ordinary accept helper

Active in YR: Conditional. In the Choose Map callback, command `0x6C5` jumps to `0x005E6B63`, calls `0x005E7160`, and returns. `0x005E7160` reads listbox `0x553` current selection (`0x188`), reads item data (`0x199`), scans `DAT_00A8B8CC` for the matching record pointer, writes selected mode/index globals, updates labels, and closes/returns through shell modal plumbing.

Evidence: assembly `0x005E69C2 CMP EAX,0x6C5`, `0x005E69CD JZ 0x005E6B63`, `0x005E6B67 CALL 0x005E7160`; decompile `0x005E7160`.

### 3.4 Create Random Map `0x583` first hides the chooser and calls `0x005E8590`

Active in YR: Conditional. Command `0x583` is decoded in the same live Choose Map callback, runs pre-modal shell handling, hides the chooser with `ShowWindow(hwnd,0)`, calls `0x005E8590`, stores the return in `EBX`, and compares it with `-1`.

Evidence: assembly `0x005E69D3 SUB EAX,0x583`, `0x005E69D8 JZ 0x005E69FD`; `0x005E6A03 CALL 0x00608070`, `0x005E6A0B CALL [ShowWindow]`, `0x005E6A11 CALL 0x005E8590`, `0x005E6A18 CMP EBX,-1`, `0x005E6A1F JZ 0x005E6B47`.

If `0x005E8590` returns `-1`, the callback jumps to `0x005E6B47`, calls `0x00608260`, shows the chooser again with `ShowWindow(hwnd,5)`, and returns without list rebuild, record upsert selection, parent close, or parent selection commit.

### 3.5 Nested random-map setup accepts only result `1`

Active in YR: Conditional. `0x005E8590` calls `0x00595BC0`. If the returned value is anything other than `1`, it returns `-1` immediately. The immediate return occurs before `DAT_008316D4 = 1`, before `0x00597730("RandMap.Sed")`, before `DAT_00AC1154` replacement, and before sentinel scan/update/append.

Evidence: decompile `0x005E8590`; assembly `0x005E85C1 CALL 0x00595BC0`, `0x005E85C6 CMP EAX,0x1`, `0x005E85C9 JZ 0x005E85CF`, `0x005E85CB OR EAX,0xFFFFFFFF`, `0x005E85CE RET`; accepted side effects begin at `0x005E85D1`.

Random-map setup Cancel is one source of non-`1`: in random-map dialog WndProc `0x00596300`, command `0x5C0` writes `*puVar5 = 2` and returns `1`, making `0x00595BC0` return `2`, which `0x005E8590` converts to `-1`.

Evidence: decompile `0x00596300`, command `0x5C0`; decompile `0x00595BC0` returns `local_28[0]`.

### 3.6 Nested setup teardown can write `RandMap.img` even on cancel

Active in YR: Conditional. `0x00595BC0` teardown is outside the `0x005E8590 == 1` gate. After the setup modal exits, if `DAT_00ABE154` exists and its inner pointer is nonzero, it opens/writes `RandMap.img`, runs the save helper, frees the temporary preview wrapper, and clears `DAT_00ABE154`. This can happen even when `local_28[0]` is `2` from setup Cancel.

Evidence: decompile `0x00595BC0`: after the pump loop and `0x00622720`, it tests `DAT_00ABE154`, writes `s_RandMap_img_00829ABC`, frees the wrapper, then returns `local_28[0]`. This code is before the caller's `result == 1` check in `0x005E8590`.

Implementation implication: do not state that setup cancel has literally no side effects. The verified no-commit guarantee is narrower: cancel/failure does not save `RandMap.Sed`, does not update/append a scenario record, does not change `DAT_00AC1154`, does not commit the sentinel, and returns to the still-open chooser with the prior selection state.

### 3.7 Accepted setup saves, replaces parent preview wrapper, upserts one sentinel, and returns an index

Active in YR: Conditional. If setup result is `1`, `0x005E8590` sets `DAT_008316D4 = 1`, saves seed/options to `RandMap.Sed`, destroys/replaces `DAT_00AC1154`, loads `RandMap.img`, scans existing scenario records for filename `RandMap.Sed`, updates the existing record if found, or appends a new official min `2` / max `4` record if absent. It returns the existing or new record index.

Evidence: decompile `0x005E8590`; assembly `0x005E85D1 PUSH RandMap.Sed`, `0x005E85DB MOV byte ptr [0x008316D4],1`, `0x005E85E2 CALL 0x00597730`, `0x005E861A PUSH RandMap.img`, `0x005E8636..0x005E8645` scan/test, `0x005E866E..0x005E8683` constructor args. `0x0069ADF0` decompile compares record `+0x58` with `RandMap.Sed`.

### 3.8 Accepted `0x583` reselects the returned record and then uses ordinary accept

Active in YR: Conditional. After a non-`-1` return, the `0x583` callback rebuilds the map list, uses `0x005E70D0` to select the returned record pointer in listbox `0x553`, loads the returned record with `0x005E7BF0(returned_index)`, falls back to normal preview load if the random preview wrapper has no inner surface, restores `DAT_00A8B254` from `DAT_00AC10E0`, loads that index, and calls `0x005E7160`.

Evidence: assembly `0x005E6A25..0x005E6B41`; decompile `0x005E70D0`, `0x005E7BF0`, `0x005E7160`. Load-bearing addresses: `0x005E6AFF CALL 0x005E70D0`, `0x005E6B06 CALL 0x005E7BF0`, `0x005E6B22 MOV [0x00A8B254],ECX`, `0x005E6B28 CALL 0x005E7BF0`, `0x005E6B2F CALL 0x005E7160`.

Implementation implication: accepted Create Random Map commits as an ordinary selected scenario-record index whose file token is `RandMap.Sed`. It is not `None`, not a negative sentinel, and not a generated `.map` filename.

## 4. INI Keys

No INI key is directly read by the scoped accept/cancel/failure state-machine branch. Liveness and list admission are conditional on the selected MPModes object's random-map flag, already verified as the fifth field in `ini/mpmodesmd.ini` and consumed by `0x005D63E0`/`0x005D6350`.

| File / section | Relevant value | Evidence | Active in YR |
|---|---|---|---|
| `ini/mpmodesmd.ini:[Battle]` | id `1` fifth field `true`; random maps allowed | local INI plus prior `0x005D6350`/`0x005D7590` reports | Yes |
| `ini/mpmodesmd.ini:[FreeForAll]` | id `2` fifth field `true`; random maps allowed | same | Conditional by selected mode |
| Other stock MPModes rows | fifth field `false`; random sentinel filtered out | same | Conditional by selected mode |

## 5. Integration Points

| Integration | Behavior | Evidence | Active in YR |
|---|---|---|---|
| Parent Skirmish command `0x5AA` | Saves old selection, hides parent, opens modal, branches on result `2` | `0x006ACEE0`, `0x006AD947..0x006AD94F` | Yes |
| Choose Map callback | Routes `0x5C0`, `0x6C5`, `0x583` by low command word | `0x005E69B7..0x005E69FD` | Yes |
| Nested setup dialog | Result `1` is accept; result `2` from setup Cancel is non-accept to `0x005E8590` | `0x00595BC0`, `0x00596300` | Conditional |
| Accepted setup | Saves `RandMap.Sed`, replaces parent preview wrapper from `RandMap.img`, upserts sentinel, returns index | `0x005E8590` | Conditional |
| Accepted command commit | Selects listbox item and calls normal `0x005E7160` | `0x005E6AFF..0x005E6B41` | Conditional |
| Failure/cancel command return | Shows chooser again, parent remains hidden, committed parent selection unchanged | `0x005E6A1F -> 0x005E6B47` | Conditional |

## 6. Current Rust Implementation Status

| Surface | Current status | Evidence |
|---|---|---|
| App button branch | Missing: logs and does not call setup/upsert/commit | `src/app.rs` `ChooseMapModalButton::CreateRandomMap0x583` branch |
| Modal saved selection | Present for ordinary modal open/cancel helper | `src/ui/skirmish_shell/state/choose_map.rs::ChooseMapModalState::saved_selection` |
| Ordinary Use Map commit | Present for current modal helper; app commits `accept_selection()` | `src/app.rs::handle_choose_map_modal_mouse_up`; `commit_choose_map_selection` |
| Lower-level random sentinel helper | Present but not wired to app button; has no native accepted setup result object | `ChooseMapModalState::create_random_map` |
| Synthetic sentinel metadata | Present for file `RandMap.Sed`, official true, min `2`, max `4`, single upsert | `src/skirmish_scenarios.rs` |
| Nested setup cancel/failure state | Missing: no setup dialog/result state, no distinction between setup cancel and Choose Map cancel | `src/app.rs` log-only branch |
| Seed/options and `.SED` handoff | Missing | no `[RandomMap]` model/reader/writer found in scoped Rust scan |
| Accepted setup preview lifecycle | Partial/missing: sentinel preview source exists in current broader codebase, but app has no accepted setup lifecycle to write/refresh `RandMap.img` at the native boundary | `src/app_skirmish_shell_render` / prior current-Rust docs |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Parent `0x5AA -> 0x6B` result split | verified | `0x006ACEE0`, `0x006AD947..0x006AD94F` | none for state split |
| Choose Map Cancel `0x5C0` | verified | `0x005E69E7..0x005E69EC` | none |
| Choose Map Use Map `0x6C5` | verified | `0x005E69C2`, `0x005E6B63..0x005E6B67`, `0x005E7160` | none for state commit |
| Choose Map Create Random Map `0x583` | verified | `0x005E69D3..0x005E6A1F` | none for state gate |
| `0x005E8590` accepted-result gate | verified | decompile/assembly `0x005E85C1..0x005E85CE` | none |
| Random-map setup Cancel result | verified | `0x00596300` command `0x5C0`; `0x00595BC0` return | none |
| Setup teardown temp preview write | verified | `0x00595BC0` | runtime screenshot impact of cancel-after-preview |
| Sentinel upsert and returned index | verified | `0x005E8590`, `0x0069ADF0` | digest/source consumer outside scope |
| Post-success reselect/accept path | verified | `0x005E6A25..0x005E6B41`, `0x005E70D0`, `0x005E7160` | exact listbox paint outside scope |
| Parent accepted load failure restore | verified by sibling + spot-check | `0x006ADA7D..0x006ADB52` | none for this state-machine boundary |
| Rust delta | verified by scoped file reads | `src/app.rs`, `src/ui/skirmish_shell/state/choose_map.rs`, `src/skirmish_scenarios.rs` | implementation not performed |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-01 - Is command 0x583 active in standard YR Choose Map? -> Yes, conditionally on the live dialog button command; no TS-only gate found in the parent/callback chain.` (evidence: `0x006AD947`, `0x005E68A0`, `0x005E69D3..0x005E6A11`)
- `[RESOLVED] OQ-02 - What closes the parent Choose Map modal as cancel? -> Choose Map command 0x5C0 sets modal result 2; parent result 2 restores saved token/index.` (evidence: `0x005E69E7..0x005E69EC`, `0x006AD94C..0x006AD961`)
- `[RESOLVED] OQ-03 - What closes the parent Choose Map modal as accept? -> Use Map 0x6C5 and accepted 0x583 both reach ordinary accept helper 0x005E7160.` (evidence: `0x005E6B63..0x005E6B67`, `0x005E6B2F`)
- `[RESOLVED] OQ-04 - What result accepts nested random-map setup? -> Exactly result 1 from 0x00595BC0.` (evidence: `0x005E85C1..0x005E85CE`)
- `[RESOLVED] OQ-05 - What happens on nested setup Cancel? -> Random-map dialog command 0x5C0 writes result 2; 0x005E8590 returns -1; chooser is shown again and no sentinel/selection commit occurs.` (evidence: `0x00596300`, `0x005E85C6..0x005E85CE`, `0x005E6A1F -> 0x005E6B47`)
- `[RESOLVED] OQ-06 - Can nested setup cancel still write RandMap.img? -> Yes, 0x00595BC0 teardown writes the temporary preview to RandMap.img when DAT_00ABE154 exists and has an inner surface, before returning the result.` (evidence: `0x00595BC0`)
- `[RESOLVED] OQ-07 - When is RandMap.Sed written? -> Only after nested setup result 1, before sentinel scan/update/append.` (evidence: `0x005E85D1..0x005E85E2`)
- `[RESOLVED] OQ-08 - When is DAT_00AC1154 replaced by RandMap.img? -> Only after nested setup result 1 inside 0x005E8590.` (evidence: `0x005E85E7..0x005E8626`)
- `[RESOLVED] OQ-09 - What index is returned on success? -> Existing or newly appended ordinary scenario-record index for record filename RandMap.Sed.` (evidence: `0x005E8636..0x005E871F`, `0x0069ADF0`)
- `[RESOLVED] OQ-10 - Does accepted 0x583 use a special commit token? -> No; it reselects listbox item data and calls ordinary 0x005E7160.` (evidence: `0x005E70D0`, `0x005E7160`, `0x005E6B2F`)
- `[RESOLVED] OQ-11 - What if accepted parent selected-record load fails later? -> Parent accepted path restores old index/token and skips normal label/preview refresh.` (evidence: sibling report plus spot-check `0x006ADA82`, `0x006ADB4B`, `0x006ADB52`)
- `[DEFERRED] OQ-12 - Exact random-map setup controls/options state before result 1.` (category: `out-of-scope`; reason: assigned to sibling slot; next-step-if-pursued: investigate random-map setup dialog controls/options)
- `[DEFERRED] OQ-13 - Exact RandMap.Sed serialized layout.` (category: `out-of-scope`; reason: assigned to sibling slot; next-step-if-pursued: investigate writer/reader full layout)
- `[DEFERRED] OQ-14 - Exact launch generation after selected .SED.` (category: `out-of-scope`; reason: assigned to sibling slot; next-step-if-pursued: investigate .SED launch branch)
- `[DEFERRED] OQ-15 - Runtime screenshot impact if setup writes RandMap.img and then cancels.` (category: `needs-runtime-debugger`; reason: static evidence proves file write/free path but not user-visible screenshot timing; next-step-if-pursued: generate preview, cancel setup, inspect chooser/parent preview and file timestamp in native)

## 9. Visual/UI Composition Ledger

This report is state-machine focused. It does not claim full modal paint composition. The following ledger covers only visual-state handoffs relevant to accept/cancel/failure.

| Order | Function / address | Condition / flag proof | Asset / frame | Rect / anchor | Palette / convert | Active for target? | Role |
|---|---|---|---|---|---|---|---|
| 1 | Choose Map callback `0x005E69FD` | command `0x583` | existing chooser surface | existing modal HWND | existing shell | Conditional | hide chooser before nested setup |
| 2 | `0x00595BC0` teardown | `DAT_00ABE154 != 0` and inner surface nonzero | `RandMap.img` | file output, not screen rect | temp preview wrapper | Conditional | temporary generated preview file write, even on setup cancel |
| 3 | `0x005E8590` accepted branch | nested result `1` | `RandMap.img` | parent/chooser preview wrapper `DAT_00AC1154` | wrapper load path | Conditional | accepted random preview source |
| 4 | Choose Map callback `0x005E6B47` | nested result converted to `-1` | no new parent preview | chooser HWND | existing shell | Conditional | show chooser again after setup cancel/failure |
| 5 | Choose Map callback `0x005E6B2F` | accepted setup returned an index | selected record's preview state | ordinary accept helper | ordinary shell | Conditional | commit selected sentinel through normal accept |

Asset role matrix:

| Asset | Loaded | Drawn | Visible in target | Content/preview | Chrome/container | Overlay | Transition-only | Inactive | Evidence |
|---|---|---|---|---|---|---|---|---|
| `RandMap.Sed` | saved on setup result `1` | no | no | seed/options handoff | no | no | no | setup cancel/failure | `0x005E85D1..0x005E85E2` |
| `RandMap.img` | temporary write possible on setup teardown; parent preview load only on setup result `1` | later paint, outside this slice | conditional | preview | no | no | no | not gameplay terrain | `0x00595BC0`, `0x005E861A..0x005E8626` |

## 10. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Nested setup Cancel/failure returns to still-open chooser and preserves committed parent selection; no sentinel/selection commit occurs | `0x00596300` `0x5C0`; `0x005E85C1..0x005E85CE`; `0x005E6A1F -> 0x005E6B47` | missing | `src/app.rs`; `src/ui/skirmish_shell/state/choose_map.rs` | Model Create Random Map as a nested setup result distinct from Choose Map Cancel; on cancel/failure keep modal open and leave committed selection/launch token unchanged | Open Choose Map on map A, highlight map B, press Create Random Map, cancel setup: chooser is still open and Start after closing/canceling still uses map A unless ordinary Use Map commits otherwise | `choose_map_create_random_map_setup_cancel_preserves_committed_selection` | Do not close the Choose Map modal or call `commit_choose_map_selection` on nested setup cancel |
| Accepted setup result `1` writes `RandMap.Sed`, replaces parent preview wrapper from `RandMap.img`, upserts one sentinel, returns its ordinary record index, then commits through `0x005E7160` | `0x005E85C6`, `0x005E85D1..0x005E8626`, `0x005E8636..0x005E871F`, `0x005E6AFF..0x005E6B2F` | app branch missing; lower-level sentinel upsert partial/present | `src/app.rs`; `src/skirmish_scenarios.rs`; preview cache/renderer | On accepted setup, create/update one sentinel and commit it through the same selected-record path as Use Map; selected launch file becomes `RandMap.Sed` | Accept setup after pressing Create Random Map: modal closes like Use Map, selected map file is `RandMap.Sed`, and one sentinel exists with native metadata | `choose_map_create_random_map_accept_commits_randmap_sed_through_use_map_path` | Do not use `None`, a negative index, display-only state, or a generated `.map` filename |
| Setup teardown may write `RandMap.img` even when setup later returns non-`1`, but parent preview wrapper `DAT_00AC1154` is only replaced after result `1` | `0x00595BC0`; `0x005E85C1..0x005E8626` | unchecked/missing lifecycle | random-map setup preview cache; `src/app_skirmish_shell_render` | Keep temp preview/file lifecycle separate from committed parent preview/cache; cancel may discard temp UI state without selecting it | Generate setup preview then cancel: no `RandMap.Sed` commit and parent/chooser committed preview remains previous selection; temp preview file/cached bytes do not become launch terrain | `choose_map_create_random_map_cancel_does_not_promote_temp_preview` | Do not equate `RandMap.img` existence with accepted random-map selection |

## Negative Facts / Do Not Do

- Do not treat `0x583` as TS-only legacy. Active in YR: No for that negative; it is live in standard YR Choose Map when clicked. Evidence: `0x006AD947 -> 0x005E68A0 -> LAB_005E6920`, `0x005E69D3..0x005E6A11`.
- Do not commit on Create Random Map button click alone. Active in YR: No; commit side effects require nested setup result `1`. Evidence: `0x005E85C1..0x005E85CE`.
- Do not treat nested random-map setup Cancel as parent Choose Map Cancel. Active in YR: No; nested cancel returns `-1` to the `0x583` branch, which shows the chooser again. Evidence: `0x00596300`, `0x005E6A1F -> 0x005E6B47`.
- Do not say setup cancel has no side effects without scoping it. Active in YR: No; teardown may write `RandMap.img` from `DAT_00ABE154`. Evidence: `0x00595BC0`.
- Do not append duplicate `RandMap.Sed` records. Active in YR: No; native scans by `record+0x58 == RandMap.Sed` and updates existing record. Evidence: `0x005E8636..0x005E871F`, `0x0069ADF0`.

## Remaining Uncertainty

- Runtime screenshot/file-timestamp behavior after generating a random preview and canceling setup remains unobserved. Static evidence proves the temporary `RandMap.img` write/free path but not its user-visible timing.
- Full random-map setup controls/options, full `.SED` layout, preview generation details, and launch generation are intentionally deferred to sibling swarm slots.

## Stale Docs / Follow-up Docs

Replace any unqualified wording equivalent to:

> `0x005E8590` returns `-1` and no side effects occur when the random-map dialog is canceled.

with:

> `0x005E8590` returns `-1` unless the random-map setup pump returns exactly `1`; in that non-accept path the `0x583` chooser callback does not save `RandMap.Sed`, does not update/append/select the sentinel, does not replace the parent preview wrapper, and returns to the still-open chooser. However, `0x00595BC0` teardown may still write/free a temporary generated preview as `RandMap.img` if the setup dialog produced one before cancel/failure, so "no side effects" must be scoped to selection/sentinel/parent-preview commit state.

Replace any wording that collapses setup cancel into Choose Map cancel with:

> Choose Map Cancel `0x5C0` returns modal result `2` to the Skirmish parent and restores the saved parent selection. Random-map setup Cancel also writes result `2` inside the nested setup dialog, but `0x005E8590` converts non-`1` to `-1`; the `0x583` branch then shows the Choose Map dialog again and does not close/commit the parent modal.

## Sources

- Fresh read-only Ghidra decompile: `0x006ACEE0`, `0x005E68A0`, `0x005E8590`, `0x00595BC0`, `0x00596300`, `0x005E70D0`, `0x005E7160`, `0x005E7BF0`, `0x0069ADF0`, `0x005D63E0`.
- Fresh read-only Ghidra assembly context: `0x006AD947`, `0x006AD94C`, `0x006AD95B`, `0x006AD961`, `0x006ADA7D`, `0x006ADA82`, `0x006ADB4B`, `0x006ADB52`, `0x005E69B7`, `0x005E69C2`, `0x005E69D3`, `0x005E69E7`, `0x005E69EC`, `0x005E69FD`, `0x005E6A11`, `0x005E6A18`, `0x005E6A1F`, `0x005E6AFF`, `0x005E6B06`, `0x005E6B22`, `0x005E6B28`, `0x005E6B2F`, `0x005E6B47`, `0x005E6B51`, `0x005E6B63`, `0x005E6B67`.
- Prior docs reconciled: `SKIRMISH_CHOOSE_MAP_ACCEPT_CANCEL_SIDE_EFFECTS_GHIDRA_REPORT.md`, `SKIRMISH_CREATE_RANDOM_MAP_0X583_SETUP_PATH_GHIDRA_REPORT.md`, `SKIRMISH_CREATE_RANDOM_MAP_0X583_IMPLEMENTATION_CONTRACT_GHIDRA_REPORT.md`, `SKIRMISH_RANDMAP_SED_RANDOM_MAP_BEHAVIOR_GHIDRA_REPORT.md`.
- Rust scan: `src/app.rs`, `src/ui/skirmish_shell/state/choose_map.rs`, `src/skirmish_scenarios.rs`, `src/app_skirmish_shell_render`.
- INI checked: `ini/mpmodesmd.ini`.
