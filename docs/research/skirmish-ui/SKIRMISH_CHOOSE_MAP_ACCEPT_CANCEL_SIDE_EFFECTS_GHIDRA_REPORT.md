# Skirmish Choose Map Accept/Cancel Side Effects - Ghidra Research Report

**Address(es):** `0x006ACEE0`, `0x005E7160`, `0x005E6520`, `0x006ADDF0`, `0x006ADF00`, `0x006AE080`, `0x005E7BF0`, `0x005E74E0`, `0x006AE3F0`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** parent-side effects after offline Skirmish Choose Map modal return: saved selection restore, accepted selection commit, loader-failure restore, capacity clamp, opponent row show/hide/reset effects, preview replacement/invalidation, and repaint handoff order.  
**Non-Scope:** modal visual layout, map list population/source order, full `MPModesMD` construction, random map generation after launch, WOL/non-offline variants.  
**Confidence:** High for the claimed offline Skirmish parent slice.  
**Active in YR:** Yes. Standard offline Skirmish reaches `0x006ACEE0` from dialog proc `0x006AE3F0` on `WM_COMMAND`; command `0x5AA` enters this branch without a TS-only gate.

## 1. Overview

Retail Choose Map is a modal transaction. The parent Skirmish dialog saves the old selected mode token/index, hides itself, waits for the chooser modal, then either restores the saved selection on cancel or rebuilds setup state around the newly committed selection on accept.

The critical implementation detail is ordering: accepted capacity/player-row rebuild happens before the parent is shown and before selected-record load succeeds. If selected-record load then fails, the old selection globals are restored and the function returns without map/mode label refresh or preview invalidation.

## 2. Key State And Controls

| Item | Meaning | Evidence | Active in YR |
|---|---|---|---|
| `DAT_00A8B250` | selected mode/category token | saved at `0x006ACEE0`, committed in `0x005E7160`, restored at `0x006AD95B` / `0x006ADB52` | Yes |
| `DAT_00A8B254` | selected scenario record index into `DAT_00A8B8CC` | saved in `EBX`, committed in `0x005E7160`, restored at `0x006AD961` / `0x006ADB4B` | Yes |
| `DAT_00A8B8E0` | selected map file/path string | copied by selected-record load `0x005E7BF0` | Yes |
| `DAT_00AC1154` | 4-byte preview wrapper pointer; wrapper `+0` owns inner surface | destroyed/replaced in `0x005E74E0` and random preview branch | Yes |
| `0x5AA` | parent Choose Map button | command branch in `0x006ACEE0` | Yes |
| `0x6EC` / `0x5A8` | game type and map title static controls | refreshed by `0x005E2EF0` / `0x005E2F60` after accepted load success | Yes |
| `0x50B,0x50E,0x516,0x51A,0x51B,0x51C,0x51D` | seven AI row type combo controls | counted/hidden/shown by row helpers | Yes |

## 3. Core Logic

### 3.1 Pre-modal save/hide

Active in YR: Yes. Evidence: decompile of `0x006ACEE0`; assembly context `0x006AD947` shows `CALL 0x005E68A0` followed by `CMP EAX,0x2`.

For `0x5AA`, the parent:

1. Saves old `DAT_00A8B254` in `EBX` and old `DAT_00A8B250` in a stack local.
2. Copies current `DAT_00A8B8E0` into `DAT_00A8B322` through a stack buffer before opening the modal.
3. Calls shell pre-modal helper `0x00608070`.
4. Calls `ShowWindow(parent, 0)`.
5. Calls modal wrapper `0x005E68A0`.
6. Compares modal result with literal `2`.

### 3.2 Cancel/back result `2`

Active in YR: Conditional. This path is active when chooser returns result `2`. Evidence: assembly `0x006AD94C..0x006AD976`; decompile of `0x006ACEE0`.

Cancel order:

1. `DAT_00A8B250 = saved_token`.
2. `DAT_00A8B254 = saved_index`.
3. Call selected-record load `0x005E7BF0(saved_index)`.
4. Call preview refresh `0x005E74E0(parent)`.
5. `ShowWindow(parent, 5)`.
6. Then run the same random-vs-normal preview branch used by this parent path; normal records invalidate through `0x005E74E0`, random records replace the wrapper with `RandMap.img` and call `InvalidateRect(parent, NULL, FALSE)` twice in the cancel branch.

Player-visible consequence: cancel must leave the previous map/mode selected and refresh/paint that restored preview. It must not commit the highlighted chooser row.

### 3.3 Accepted result not `2`

Active in YR: Yes. Evidence: `0x006AD94F JNZ 0x006ADA21`; decompile of `0x005E7160` shows accept commits `DAT_00A8B250/254` before closing.

Accepted parent order:

1. `0x005E6520(DAT_00A8B254)` computes selected-map player capacity.
2. Calls selected mode vtable `+0x04`. If it returns true, candidate capacity is clamped to `min(map_capacity, *(DAT_00A8B230 + 0x11E4))`; if false, capacity remains `map_capacity`. Evidence: assembly `0x006ADA36..0x006ADA4F`.
3. `0x004E4FC0(capacity)` rewrites the per-slot state table: slots before `capacity` become `-1`, slots from `capacity` through 8 become `-2`. Evidence: decompile `0x004E4FC0`.
4. `0x004E5310()` and `0x004E5D60()` rebuild color/country/start/team combo backing state for all 8 slots. Active in YR: Yes; standard offline falls through the non-WOL/non-LAN helper paths.
5. `0x006ADDF0(parent, old_index, new_index)` adjusts visible opponent rows by capacity delta and resets team item data according to selected mode.
6. `ShowWindow(parent, 5)`.
7. `0x005E7BF0(DAT_00A8B254)` loads selected-record fields.
8. If load fails, restore old `DAT_00A8B254` then old `DAT_00A8B250`, and return. Evidence: `0x006ADA82 JZ 0x006ADB45`, `0x006ADB4B`, `0x006ADB52`.
9. If load succeeds, refresh mode label `0x6EC`, map label `0x5A8`, dependent control state `0x006ACD60`, then preview.

### 3.4 Row rebuild / capacity delta helper

Active in YR: Yes. Evidence: accepted branch calls `0x006ADDF0` at `0x006ADA6D`; decompile of `0x006ADDF0`, `0x006ADF00`, and `0x006AE080`.

`0x006ADDF0(parent, old_index, new_index)`:

- If `old_index == -1`, old capacity is treated as `8`; otherwise old capacity is `0x005E6520(old_index)`.
- New capacity is `0x005E6520(new_index)`.
- `delta = new_capacity - old_capacity`.
- If both old and new indices are valid and both scenario records are `RandMap.Sed`, it hides from `new_capacity` onward and then shows rows through `new_capacity - 1`.
- If `delta > 0`, it shows the newly available rows by calling `0x006ADF00(parent, old_capacity + 1, delta)`.
- If `delta < 0`, it hides rows from `new_capacity + 1` upward by calling `0x006AE080(parent, new_capacity + 1)`.
- For rows 1..7, it resets team combo item data through `0x004E5ED0`: `-2` when selected mode object is null or its byte `+0x3C` is false, otherwise `3`.

`0x006ADF00` shows six child controls for each affected AI row: row type combo, side, color, start, team, and one more row-owned control, then calls `0x006ADC20` to refresh row-dependent state. `0x006AE080` first selects the `-1` item in row-type combos where present, calls `0x006ADC20`, then hides the same row-owned controls.

### 3.5 Selected-record load and failure boundary

Active in YR: Yes. Evidence: accepted branch calls `0x005E7BF0` at `0x006ADA7D`; decompile of `0x005E7BF0`.

`0x005E7BF0(index)` returns false when `index == -1`, out of range, or the selected map file cannot be opened/resolved. On success it copies:

- record `+0x00` display title to `DAT_00A8B322`;
- record `+0x15C` digest to `DAT_00A8BAE2`;
- record `+0x58` file/path to `DAT_00A8B8E0`;
- `DAT_00A8B8E0` to `ScenarioClass + 0x125C`;
- record `+0x17C` official flag to `DAT_00A8BB08`;
- capacity/mask into `DAT_00A8BB0C`, clamped by selected mode vtable `+0x98` if that returns a value other than `0xFFFFFFFF`;
- file vtable `+0x2C` result into `DAT_00A8BB04` twice.

Failure on the accepted parent path restores old token/index but does not undo the earlier row rebuild/show. That boundary matters for parity if Rust exposes an accepted map that later fails to load.

### 3.6 Preview replacement and repaint

Active in YR: Yes for normal stock maps; Conditional for `RandMap.Sed`. Evidence: decompile `0x005E74E0`; assembly `0x006ADAC3..0x006ADB33`; decompile `0x006AE3F0` paint branch.

Normal map accepted path calls `0x005E74E0(parent)` after label/control refresh. `0x005E74E0` destroys and frees any existing `DAT_00AC1154`, clears it, opens `DAT_00A8B8E0`, allocates a 4-byte wrapper on success, initializes wrapper `+0` to null through `0x006406E0`, loads preview content, then calls `InvalidateRect(parent, NULL, FALSE)` if a wrapper exists.

Random-map records are detected by `0x0069ADF0(record)` comparing record `+0x58` to literal `RandMap.Sed`. The parent random branch destroys the old wrapper if present, allocates/zeroes a new wrapper, loads `RandMap.img`, falls back to `0x005E74E0` if wrapper `+0` remains null, then invalidates parent.

The parent `WM_PAINT` path `0x006AE3F0` does the actual draw later: if `DAT_00AC1154 != 0`, it gets child `0x468`, calls `0x006067A0`, conditionally calls `DrawStartPositions @ 0x00640710`, then validates the parent rect. The Choose Map return path invalidates; it does not directly paint the preview.

## 4. INI Keys

No INI keys are directly read in this parent-side return slice. Capacity uses selected scenario data via `0x005E6520`, which reads `[Waypoints]` entries `0..7` from the selected map and falls back to `[RandomMap] NumPlayers`, defaulting to `8` when the random-map key is zero/missing. Active in YR: Yes; evidence decompile `0x005E6520`.

## 5. Integration Points

| Integration point | Behavior | Evidence | Active in YR |
|---|---|---|---|
| Parent command dispatch | `WM_COMMAND` reaches `0x006ACEE0`; command id comes from low word, notification from high word | `0x006AE3F0` decompile | Yes |
| Modal accept commit | `0x005E7160` reads listbox `0x553` selected item data, finds matching `DAT_00A8B8CC` record, writes `DAT_00A8B250/254` | `0x005E7160` decompile | Yes |
| Capacity and row rebuild | accepted branch recomputes capacity and calls row helpers before show and selected-record load | `0x006ADA21..0x006ADA7D` | Yes |
| Preview paint | invalidation triggers later `WM_PAINT`; draw is gated by `DAT_00AC1154` | `0x006AE3F0`, `0x00640710` | Yes |

## 6. Current Rust Implementation Status

Rust currently has the button/action identity, but not the modal transaction:

- `src/ui/skirmish_shell/state.rs:965` cycles `selected_map_idx` in place for `ChooseMap`.
- `src/app.rs:564` calls `apply_action`; `src/app.rs:585` treats `ChooseMap` as swallowed/no-op after `apply_action`.
- `src/ui/skirmish_shell/state.rs:441` derives start-position choices from the current map's waypoint count, but there is no accept-time row rebuild or restoration boundary.
- `src/ui/skirmish_shell/state.rs:823` validates selected map capacity on Start, not immediately after accepted map change.
- `src/app_skirmish_shell_render.rs:1633` lazily rebuilds preview texture from `selected_map_idx`, but there is no explicit accept/cancel invalidation or load-failure restore.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Parent `0x5AA` modal return split | verified | `0x006ACEE0`, `0x006AD947..0x006ADA21` | none for offline Skirmish |
| Cancel restore | verified | `0x006AD94C..0x006AD976` | double invalidation reason for random cancel is not user-visible beyond repaint |
| Accepted capacity clamp | verified | `0x006ADA27..0x006ADA4F`, `0x005E6520` | concrete mode `+0x04` meaning belongs to mode-object report |
| Row show/hide/reset helper | verified | `0x006ADDF0`, `0x006ADF00`, `0x006AE080` | exact child ID naming for one row-owned helper control not required for Rust handoff |
| Selected-record load failure boundary | verified | `0x005E7BF0`, `0x006ADA7D..0x006ADB52` | none |
| Normal preview replacement/invalidation | verified | `0x005E74E0`, `0x006ADB31..0x006ADB33` | PreviewPack decode internals out of scope |
| Random preview replacement | verified | `0x0069ADF0`, `0x006ADAC3..0x006ADB1E` | random map generation out of scope |
| Paint handoff | verified | `0x006AE3F0`, `0x00640710` | preview marker geometry out of scope |
| List population/source order | deferred | sibling reports | out-of-scope by swarm slot |
| Modal visual layout | deferred | sibling slot | out-of-scope by swarm slot |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-1 - Is the parent branch active in standard YR Skirmish? -> Yes, `WM_COMMAND` routes to `0x006ACEE0`; command `0x5AA` enters Choose Map path.` (evidence: `0x006AE3F0`, `0x006ACEE0`)
- `[RESOLVED] OQ-2 - What is saved before modal open? -> old selected token `DAT_00A8B250`, old scenario index `DAT_00A8B254`, and current path text copied through `DAT_00A8B322`.` (evidence: `0x006ACEE0`)
- `[RESOLVED] OQ-3 - What exact result means cancel? -> result `2`; parent compares `EAX` to `2` immediately after modal return.` (evidence: `0x006AD947..0x006AD94F`)
- `[RESOLVED] OQ-4 - Does cancel commit highlighted chooser row? -> No; it restores old token/index before load/preview refresh.` (evidence: `0x006AD95B`, `0x006AD961`)
- `[RESOLVED] OQ-5 - Does accept restore old state? -> No on success; accept uses values already committed by `0x005E7160`.` (evidence: `0x005E7160`, `0x006ADA21`)
- `[RESOLVED] OQ-6 - What happens if accepted selected-record load fails? -> old index/token are restored and the branch returns before labels/preview refresh.` (evidence: `0x006ADA82`, `0x006ADB4B`, `0x006ADB52`)
- `[RESOLVED] OQ-7 - Is row rebuild before or after showing setup? -> Before; `0x004E4FC0`, `0x004E5310`, `0x004E5D60`, `0x006ADDF0` precede `ShowWindow(parent,5)`.` (evidence: `0x006ADA4F..0x006ADA75`)
- `[RESOLVED] OQ-8 - Is capacity clamped? -> Yes, conditionally by selected mode vtable `+0x04` and `DAT_00A8B230+0x11E4`.` (evidence: `0x006ADA36..0x006ADA4F`)
- `[RESOLVED] OQ-9 - What does row delta update do? -> show rows for capacity growth, hide rows for capacity shrink, and reset row type selection to `-1` before hiding.` (evidence: `0x006ADDF0`, `0x006ADF00`, `0x006AE080`)
- `[RESOLVED] OQ-10 - Does Choose Map directly paint preview? -> No; it replaces/loads preview wrapper and invalidates parent; later `WM_PAINT` draws.` (evidence: `0x005E74E0`, `0x006AE3F0`)
- `[RESOLVED] OQ-11 - Is `RandMap.Sed` special in return path? -> Yes, random records load `RandMap.img` preview wrapper instead of the selected map preview path.` (evidence: `0x0069ADF0`, `0x006ADAC3..0x006ADB1E`)
- `[DEFERRED] OQ-12 - What is the complete visual layout of the modal?` (category: `out-of-scope`; reason: assigned to visual/layout slot; next-step-if-pursued: use `SKIRMISH_CHOOSE_MAP_MODAL_VISUAL_CONTROL_LAYOUT`)
- `[DEFERRED] OQ-13 - What is exact source/list population order?` (category: `out-of-scope`; reason: assigned to source-order/list slot; next-step-if-pursued: use `SKIRMISH_SCENARIO_SOURCE_POPULATION_ORDER`)
- `[DEFERRED] OQ-14 - What does random map generation do after Start?` (category: `out-of-scope`; reason: this slice only covers chooser return preview; next-step-if-pursued: use random-map behavior report)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Choose Map opens a modal transaction; cancel restores old selection | `0x006AD947..0x006AD976` | missing; Rust cycles index in `apply_action` | `src/ui/skirmish_shell/state.rs`, `src/app.rs` | `ChooseMap` should open chooser state with saved selected map/mode; cancel restores without changing launch map | proposed test `skirmish_choose_map_cancel_restores_previous_selection` | Do not mutate committed `selected_map_idx` on highlight or button click alone |
| Accept commits selected record, then rebuilds capacity/rows before selected-record load | `0x005E7160`, `0x006ADA21..0x006ADA7D` | missing | future chooser modal state plus `SkirmishShellState` row helpers | accept should commit selected map/mode, clamp capacity, adjust active row controls/start choices, then refresh labels/preview | proposed test `skirmish_choose_map_accept_rebuilds_rows_before_preview_refresh` | Do not defer row capacity changes until Start only |
| Accepted selected-record load failure restores old token/index and skips normal label/preview refresh | `0x006ADA82..0x006ADB52` | missing/unchecked, current lazy decode just clears texture on failure | app-level chooser accept flow and preview cache | failed accepted load leaves previous committed map visible/launchable and does not replace labels with failed map | proposed test `skirmish_choose_map_accept_load_failure_restores_previous_map` | Do not leave UI committed to a map whose file failed to load |
| Preview refresh is replacement plus parent invalidation, not direct paint | `0x005E74E0`, `0x006AE3F0` | partial lazy cache keyed by selected index | `src/app_skirmish_shell_render.rs`, app screen invalidation/state | accept/cancel should invalidate/clear preview cache so next render consumes committed selection; random map needs `RandMap.img` path | proposed test `skirmish_choose_map_accept_invalidates_preview_cache_once_committed` | Do not draw preview from transient highlighted chooser selection |
| Capacity uses map waypoint count/random fallback, then optional mode clamp | `0x005E6520`, `0x006ADA36..0x006ADA4F` | partial; current uses `multiplayer_start_waypoints.len()` only | `src/app_list_maps.rs`, `src/ui/skirmish_shell/state.rs` | keep map capacity as selected-record metadata and apply mode cap where known | proposed test `skirmish_choose_map_accept_clamps_start_positions_to_selected_map_capacity` | Do not hardcode 8 for every map or only validate at launch |

## Negative Facts / Do Not Do

- Do not implement Choose Map as a next-map cycle; binary opens a modal and branches on modal result.
- Do not commit chooser highlight as the selected map; commit occurs on accept helper `0x005E7160`.
- Do not update the preview from transient chooser highlight unless a separate retail hover/list-selection preview report proves it; this parent slice refreshes on return.
- Do not treat accepted load failure as success with missing preview; binary restores old selected token/index and skips label/preview refresh.
- Do not make row capacity only a Start validation; accepted map change rebuilds row control state immediately.

## Stale Docs / Follow-up Docs

No prior claim was found to be wrong. Sharpen wording in `SKIRMISH_CHOOSE_MAP_MODAL_FLOW_GHIDRA_REPORT.md` if edited later:

- Replace "accepted path rebuilds player/combo state from the newly committed selection" with "accepted path computes selected-map capacity, optionally clamps it through selected-mode vtable `+0x04` and `DAT_00A8B230+0x11E4`, rewrites slot state via `0x004E4FC0`, rebuilds combo backing stores, applies row show/hide deltas in `0x006ADDF0`, then shows the parent before selected-record load; loader failure restores old token/index and skips label/preview refresh."

## Sources

- Ghidra read-only decompile: `0x006ACEE0`, `0x005E7160`, `0x005E6520`, `0x006ADDF0`, `0x006ADF00`, `0x006AE080`, `0x005E7BF0`, `0x005E74E0`, `0x006AE3F0`, `0x0069ADF0`, `0x00640710`, `0x006406E0`, `0x006406F0`, `0x005E2EF0`, `0x005E2F60`, `0x004E4FC0`, `0x004E5310`, `0x004E5D60`.
- Ghidra assembly contexts: `0x006AD947`, `0x006AD94C`, `0x006ADA21`, `0x006ADA27`, `0x006ADA36`, `0x006ADA49`, `0x006ADA6D`, `0x006ADA72`, `0x006ADA7D`, `0x006ADA82`, `0x006ADA90`, `0x006ADAC3`, `0x006ADB31`, `0x006ADB45`, `0x006ADB4B`.
- Prior docs checked: `SKIRMISH_CHOOSE_MAP_MODAL_FLOW_GHIDRA_REPORT.md`, `SKIRMISH_CHOOSE_MAP_LIST_POPULATION_ORDER_GHIDRA_REPORT.md`, `SKIRMISH_CHOOSE_MAP_MODAL_RETURN_CONTRACT_GHIDRA_REPORT.md`, `SKIRMISH_SELECTED_MAP_TOKEN_LOAD_CONSUMER_GHIDRA_REPORT.md`, `SKIRMISH_START_VALIDATION_FAILURE_UI_GHIDRA_REPORT.md`.
- Rust contrast scan: `src/ui/skirmish_shell/state.rs`, `src/app.rs`, `src/app_skirmish_shell_render.rs`, `src/app_list_maps.rs`.
