# Skirmish Create Random Map 0x583 Setup Path - Ghidra Research Report

**Address(es):** `0x005E6920/LAB_005E6920`, `0x005E8590`, `0x00595BC0`, `0x00596300`, `0x00597730`, `0x00597A10`, `0x005E84D0`, `0x005E70D0`, `0x005E7BF0`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** Choose Map modal command `0x583` through `0x005E8590`: random-map dialog result, seed/options state saved to `.SED`, `RandMap.img` preview wrapper replacement, sentinel record create/update, list reselect/selected-record writes, and later launch seed-load handoff.  
**Non-Scope:** random terrain formulas inside `0x00598960`, exact random-map dialog visual layout, full mode-family UI variants beyond standard offline Skirmish activity, and passive Choose Map row-preview timing already resolved by sibling reports.  
**Confidence:** High for command/setup/handoff state; Medium for per-option field names where semantics are inferred from UI strings and generator consumers.  
**Active in YR:** Conditional. The path is live in standard YR offline Choose Map when command `0x583` is enabled and clicked; generation itself only commits when the random-map dialog returns result `1`.

## Working Notes

- Target question: What exact side effects does Choose Map command `0x583` perform before later launch consumes `RandMap.Sed`?
- Non-goals: Do not drain `0x00598960` terrain formulas, listbox row paint, combo popup behavior, trackbar behavior, or passive row-preview refresh.
- Evidence needed to mark COMPLETE: callback command branch, `0x005E8590` create/update path, random-map dialog result path, seed/options save/load wrappers, selected-record writes, and Rust delta scan.
- Stop conditions: stop after `.SED` setup/handoff is proven; defer terrain-generation internals and runtime-only dialog experiments.

## 1. Overview

The `Create Random Map` button is not a passive chooser highlight path. It hides the Choose Map dialog, opens the random-map generator dialog, and only if that dialog returns `1` does it save current random-map seed/options to `RandMap.Sed`, replace the preview wrapper from `RandMap.img`, create or update one synthetic scenario record, rebuild/select the list row, load the selected token, restore the previously committed index, and call the normal Use Map accept helper.

The later launch branch does not receive a generated map filename. It receives the ordinary selected-record filename `RandMap.Sed`; `.SED` suffix detection then calls the seed-load wrapper before `0x00598960(0,0)` generates map state in memory.

## 2. Key State / Offsets

| Item | Purpose in this slice | Evidence | Active in YR |
|---|---|---|---|
| command `0x583` | Choose Map `Create Random Map` button | `0x005E69FD..0x005E6B57` | Conditional: button click |
| `0x005E8590` | command-side random sentinel setup | decompile plus assembly `0x005E8590..0x005E871F` | Conditional: returns `-1` unless dialog result is `1` |
| `DAT_00ABDFD8` | map seed/options object used by save/load wrappers | `0x005E85D6`, `0x00597A10`, `0x00596300` copy from `DAT_00ABDFD8` | Conditional |
| `DAT_00ABE050` | random-map display/name buffer, also preserved while digest is computed | `0x00596300`, `0x005E84D0`, `0x0069ACD0` | Conditional |
| `DAT_00ABE010..` | random-map option/seed fields hashed by `0x005E84D0` and consumed by generator | `0x005E84D0`, `0x00597260`, `0x005975E0`, `0x00598960` | Conditional |
| `DAT_00ABE04C` / seed offset `+0x74` | 0..0xFFFF seed value; dialog init randomizes when `-1`; generator seeds RNG from it | `0x00596300`, `0x005975E0`, `0x00598960` | Conditional |
| record `+0x58` | filename token `RandMap.Sed` | `0x0069A980`, `0x0069ADF0`, `0x005E7BF0` | Conditional |
| record `+0x15C` | digest string from `0x005E84D0` | `0x005E870E..0x005E8716`, `0x0069AD80` | Conditional |
| record `+0x17C` | official flag, set true by constructor arg | `0x005E8674..0x005E8683`, `0x0069A980` | Conditional |
| record `+0x180/+0x184` | min/max players, set `2` / `4` | `0x005E866E..0x005E8683`, `0x0069A980` | Conditional |
| `DAT_00AC1154` | preview wrapper replaced and loaded from `RandMap.img` | `0x005E85E7..0x005E8626`, `0x00641DB0` | Conditional |

## 3. Core Logic

### 3.1 Command `0x583` gates all later side effects

Active in YR: Conditional. In the live `0x6B` callback branch, command `0x583` first performs shell hide/pre-modal calls, then calls `0x005E8590`. If the return value is `-1`, the branch jumps to the cleanup/show path and does not rebuild/select a random record.

Evidence: assembly context `0x005E69FD..0x005E6A1F`; direct call `0x005E6A11 -> 0x005E8590`; compare `0x005E6A18` and `JZ 0x005E6B47`.

### 3.2 `0x005E8590` opens the random-map dialog and aborts unless result is exactly `1`

Active in YR: Conditional. `0x005E8590` chooses a callback/tick helper by `g_GameMode` (`DAT_00A8B238`) before calling `0x00595BC0`. If the dialog returns anything other than `1`, `0x005E8590` returns `-1` and the Choose Map command does not create/update `RandMap.Sed`.

Evidence: assembly `0x005E8590..0x005E85C9`; `0x00595BC0` decompile creates the dialog, pumps until `local_28[0]` changes, and returns that result.

### 3.3 Accepted random-map dialog saves the current seed/options to `RandMap.Sed`

Active in YR: Conditional. After accepted result, `0x005E8590` sets byte `DAT_008316D4 = 1`, calls `0x00597730` on `DAT_00ABDFD8` with argument `RandMap.Sed`, then replaces `DAT_00AC1154` and loads `RandMap.img`.

Evidence: assembly `0x005E85D1..0x005E8626`; `0x00597730` decompile dispatches vtable `+8` with the filename when passed a nonzero filename; launch-side `0x00597A10` dispatches vtable `+4` with the filename when loading seed/options.

### 3.4 Random-map dialog controls write deterministic option globals before save

Active in YR: Conditional. The random-map dialog WndProc `0x00596300` owns the option state. Init `0x497` seeds `DAT_00ABE04C` with `RandomRanged(0,0xFFFF)` when it was `-1`, populates combos, clamps fields, and disables OK/Create buttons until a preview/update path runs.

Evidence: `0x00596300` decompile; `0x005975E0` clamps terrain/resource/theater/size/player/seed fields; `0x00596E50` populates controls; `0x00597260` randomizes derived size-dependent fields and seed.

Important constants verified from helpers:

- type/landform offset `+0x3C` clamps `0..4`.
- theater/resources/time-style offsets `+0x38`, `+0x40`, `+0x48`, `+0x64`, `+0x68` clamp `0..3`.
- player count offset `+0x50` clamps `2..8`.
- percent-style offsets `+0x44`, `+0x4C`, `+0x54`, `+0x58`, `+0x5C`, `+0x60`, `+0x6C`, `+0x70` clamp `0..100`, except `+0x54` clamps `1..100`.
- seed offset `+0x74` clamps `0..0xFFFF`.

### 3.5 Randomize/generate buttons mutate the same seed/options buffer used by `0x005E8590`

Active in YR: Conditional. Command `0x621` in the random-map dialog randomizes selected options (`DAT_00ABE010`, `DAT_00ABE014`, `DAT_00ABE018`, `DAT_00ABE020`, `DAT_00ABE03C/40`, `DAT_00ABE04C`), reloads default display text `0xF5E`, regenerates derived fields via `0x00597260`, clamps via `0x005975E0`, clears preview pointer `DAT_00ABE154`, disables OK/Create buttons, and invalidates the dialog.

Evidence: `0x00596300` decompile command `0x621`; helper decompiles `0x00597260`, `0x005975E0`.

Command `0x620` copies the current `DAT_00ABDFD8` seed object into `DAT_00ABE150` after calling `0x00598960(1, hwnd)` and `GenerateTerrainPreview`, then posts paint. This is preview-time generation, not the later launch generation.

Evidence: `0x00596300` command `0x620`; copy loop from `DAT_00ABDFD8` into new `MapSeedClass` object; `0x00598960` decompile shows `param_2 != 0` preview updates, while launch uses `(0,0)`.

### 3.6 `0x005E8590` creates or updates exactly one sentinel record

Active in YR: Conditional. After saving seed/options and replacing preview, `0x005E8590` scans `DAT_00A8B8CC[0..DAT_00A8B8D8)` and calls `0x0069ADF0` on each record. `0x0069ADF0` compares record `+0x58` to `RandMap.Sed`.

If found, the existing record is updated in place: record display/name gets `DAT_00ABE050` through `0x0069ACD0`, digest/source gets a freshly computed `0x005E84D0` string through `0x0069AD80`, and the existing index is returned.

If not found, a `0x1BC` record is allocated and constructed with file `RandMap.Sed`, display/name `DAT_00ABE050`, digest `0x005E84D0`, official flag `1`, no map `GameModes` list argument, min players `2`, max players `4`, then appended to the global record vector.

Evidence: decompile `0x005E8590`; assembly `0x005E8636..0x005E871F`; constructor `0x0069A980`; update helpers `0x0069ACD0`, `0x0069AD80`.

### 3.7 The command branch reselects and then accepts through ordinary selected-record paths

Active in YR: Conditional. When `0x005E8590` returns an index, command `0x583` rebuilds the map list for the currently selected mode, reselects the returned record pointer with `0x005E70D0`, calls `0x005E7BF0(returned_index)`, checks whether `DAT_00AC1154+0` is null and falls back to `0x005E74E0` if needed, restores `DAT_00A8B254 = DAT_00AC10E0`, calls `0x005E7BF0(DAT_00AC10E0)`, and then calls `0x005E7160`.

This odd ordering means the command can load the generated preview/list row for the new sentinel but the final commit is still the listbox current selection through the normal accept helper. `0x005E7160` scans listbox item data back to a scenario-record index and writes `DAT_00A8B254`.

Evidence: assembly `0x005E6A25..0x005E6B41`; `0x005E70D0` decompile; `0x005E7BF0` decompile; `0x005E7160` decompile.

### 3.8 Later launch consumes `.SED` by loading seed/options, not by reading a normal map INI

Active in YR: Conditional. `0x005E7BF0` copies record `+0x58` into `DAT_00A8B8E0` and `ScenarioClass+0x125C`. The launch branch in `ScenarioClass__Read_Scenario @ 0x00684620` detects `.SED`, then calls `0x00597A10` on `DAT_00ABDFD8` with the local filename before calling `0x00598960(0,0)`.

Evidence: `0x005E7BF0` decompile; `0x00597A10` decompile; sibling report `SKIRMISH_RANDOM_MAP_BRANCH_AFTER_SELECTED_MAP_LOAD_GHIDRA_REPORT.md` evidence `0x00684961..0x00684990`.

## 4. INI Keys

No new INI key is read directly by `0x583` or `0x005E8590`. List admission still depends on the selected MPModes random-map flag (`mode+0x34`, fifth roster field), already verified by `0x005D63E0` and `0x005D6350`. Active in YR: Conditional by selected mode; evidence: `SKIRMISH_RANDMAP_SED_RANDOM_MAP_BEHAVIOR_GHIDRA_REPORT.md`.

## 5. Integration Points

| Integration | Behavior | Evidence | Active in YR |
|---|---|---|---|
| Choose Map callback | `0x583` calls `0x005E8590`; `-1` skips list/accept side effects | `0x005E69FD..0x005E6B57` | Conditional |
| Random-map dialog | returns `1` only when accepted; cleans preview/seed objects after modal exits | `0x00595BC0`, `0x00596300` | Conditional |
| Seed save | accepted command writes `DAT_00ABDFD8` state to `RandMap.Sed` via vtable `+8` | `0x005E85D1..0x005E85E2`, `0x00597730` | Conditional |
| Sentinel record | update-or-append by filename compare `RandMap.Sed` | `0x005E8636..0x005E871F`, `0x0069ADF0` | Conditional |
| Preview | `DAT_00AC1154` replaced and loaded from `RandMap.img` | `0x005E85E7..0x005E8626`, `0x00641DB0` | Conditional |
| Launch | `.SED` load calls `0x00597A10` before `0x00598960(0,0)` | `0x00597A10`; sibling `0x00684620` report | Conditional |

## 6. Current Rust Implementation Status

Rust now has a random sentinel shape but not the native command-side seed/options contract.

| Rust area | Current status | Evidence |
|---|---|---|
| sentinel record kind | present | `src/skirmish_scenarios.rs::SkirmishScenarioKind::RandomMapSentinel` |
| `RandMap.Sed` filename | present | `src/skirmish_scenarios.rs::RANDMAP_SED` |
| upsert one sentinel | present | `src/skirmish_scenarios.rs::upsert_random_map_sentinel` |
| random-mode filter | present | `src/skirmish_scenarios.rs::record_matches_mode` |
| command state helper | partial; only upserts display name and refreshes modal records | `src/ui/skirmish_shell/state.rs::ChooseMapModalState::create_random_map` |
| native sentinel fields | mismatch: Rust sentinel has `official=false`, no min/max `2/4`, no digest/source, no random seed/options object | `src/skirmish_scenarios.rs::random_map_sentinel` |
| preview | mismatch: current preview path decodes concrete map preview and has no `RandMap.img` branch | `src/app_skirmish_shell_render.rs::ensure_selected_preview_texture` |
| launch | mismatch: selected map file is loaded as a normal map token; no `.SED` random-generation route | `src/app_list_maps.rs::load_map_by_name_or_path`, `src/app_init.rs::load_map`, `src/ui/skirmish_shell/state.rs::launch_settings` |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `0x583` callback branch | verified | `0x005E69FD..0x005E6B57` | none for setup path |
| `0x005E8590` result gate | verified | `0x005E8590..0x005E85C9` | exact non-offline callback meaning not needed |
| random-map dialog WndProc option writes | verified for setup fields | `0x00596300`, `0x00596C70`, `0x00596E50`, `0x00597260`, `0x005975E0` | visual layout out-of-scope |
| seed save/load wrappers | verified | `0x00597730`, `0x00597A10` | concrete on-disk `.SED` byte format not decoded |
| digest helper `0x005E84D0` | verified at formula level needed for record update | `0x005E84D0` | exact user-facing use of digest outside list/session not traced |
| sentinel create/update | verified | `0x005E8636..0x005E871F`, `0x0069A980`, `0x0069ACD0`, `0x0069AD80` | none |
| `RandMap.img` preview load | verified | `0x005E85E7..0x005E8626`, `0x00641DB0` | exact bitmap pixels not sampled |
| launch handoff | verified by current and sibling report | `0x005E7BF0`, `0x00597A10`, sibling `0x00684620` | generator formulas slot 1 |
| Rust delta | verified by Codegraph/rg | listed in Section 6 | implementation not performed |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-01 - Is command 0x583 live in standard offline YR? -> Yes, as a conditional Choose Map button branch.` (evidence: `0x005E69FD..0x005E6B57`)
- `[RESOLVED] OQ-02 - What aborts the command path? -> `0x005E8590` returns `-1` unless `0x00595BC0` returns exactly `1`.` (evidence: `0x005E85C1..0x005E85CE`)
- `[RESOLVED] OQ-03 - Does accepted setup save seed/options before record creation? -> Yes, `DAT_00ABDFD8` vtable `+8` receives `RandMap.Sed` before preview/record work.` (evidence: `0x005E85D1..0x005E85E2`, `0x00597730`)
- `[RESOLVED] OQ-04 - Which buffer names the generated map record? -> `DAT_00ABE050` is copied to record display/name, with default/randomize text loaded from string id `0xF5E`.` (evidence: `0x00596300`, `0x0069ACD0`)
- `[RESOLVED] OQ-05 - Does command create duplicate sentinel records? -> No; it scans for `record+0x58 == RandMap.Sed` and updates in place if found.` (evidence: `0x005E8636..0x005E871F`, `0x0069ADF0`)
- `[RESOLVED] OQ-06 - What fields are set on a new sentinel record? -> file `RandMap.Sed`, name `DAT_00ABE050`, digest `0x005E84D0`, official `1`, min `2`, max `4`.` (evidence: `0x005E866E..0x005E8683`, `0x0069A980`)
- `[RESOLVED] OQ-07 - Does setup load map PreviewPack? -> No, it replaces `DAT_00AC1154` and loads `RandMap.img`.` (evidence: `0x005E85E7..0x005E8626`, `0x00641DB0`)
- `[RESOLVED] OQ-08 - Does the command commit by special token? -> No, it reselects listbox row and calls ordinary accept `0x005E7160`.` (evidence: `0x005E6B04..0x005E6B41`, `0x005E7160`)
- `[RESOLVED] OQ-09 - What launch loader reads the saved seed/options? -> `.SED` launch calls `0x00597A10` vtable `+4` before generator `0x00598960(0,0)`.` (evidence: `0x00597A10`, sibling `0x00684961..0x00684990`)
- `[RESOLVED] OQ-10 - Is this a TS-only legacy path? -> No; it is reached from standard YR shell UI and random-map dialog, gated only by user command/result/mode state.` (evidence: `0x005E69FD..0x005E6B57`, `0x00595BC0`)
- `[DEFERRED] OQ-11 - Exact terrain formulas inside `0x00598960`.` (category: out-of-scope; reason: slot 1 owns generator internals; next-step-if-pursued: drain `0x00598960` callees)
- `[DEFERRED] OQ-12 - Exact `.SED` serialized byte layout.` (category: bounded-cost-too-high; reason: save/load vtable behavior and handoff are proven but file format requires object-method deep dive; next-step-if-pursued: resolve `DAT_00ABDFD8` concrete vtable)
- `[DEFERRED] OQ-13 - Exact localized English string for id `0xF5E`.` (category: bounded-cost-too-high; reason: binary uses the id, display-buffer ownership is proven; next-step-if-pursued: decode CSF key/value)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Accepted `0x583` saves a seed/options object to `RandMap.Sed` before sentinel commit | `0x005E85D1..0x005E85E2`, `0x00597730`, `0x00597A10` | missing | future random-map setup state; `src/ui/skirmish_shell/state.rs`; launch path in `src/app_init.rs` / `src/app_transitions.rs` | Store a deterministic random-map setup object alongside the sentinel and make launch route `.SED` through generation rather than normal map load | Press Create Random Map, accept, Start, and assert launch uses stored setup instead of attempting filesystem parse of `RandMap.Sed` | `skirmish_create_random_map_saves_seed_options_before_launch` | Do not model random map as display-name-only sentinel state. |
| One `RandMap.Sed` scenario record is updated in place; new record fields are official true, min `2`, max `4`, digest from setup | `0x005E8636..0x005E871F`, `0x0069A980`, `0x0069ACD0`, `0x0069AD80` | partial mismatch: Rust upserts one sentinel but sets official false and min/max none; no digest/source | `src/skirmish_scenarios.rs` | Update sentinel model to carry native min/max/official and a setup digest/source field when available | Create random twice and assert one sentinel remains, display/digest update, and min/max report `2/4` | `skirmish_create_random_map_updates_single_sentinel_with_native_fields` | Do not append duplicate records or treat `RandMap.Sed` as a loose-map scan result. |
| Command-side preview wrapper is replaced from `RandMap.img`, not map `PreviewPack` | `0x005E85E7..0x005E8626`, `0x00641DB0` | missing | `src/app_skirmish_shell_render.rs`, asset/preview cache surface | Add random-sentinel preview source keyed to `RandMap.img` or an equivalent generated preview branch | After accepting random sentinel, preview cache is random-specific and does not decode concrete map preview data | `skirmish_create_random_map_uses_randmap_img_preview_source` | Do not leave the previous concrete map thumbnail visible after command accept. |
| `0x583` reselects the returned record and uses ordinary `0x005E7160` accept semantics | `0x005E6A25..0x005E6B41`, `0x005E70D0`, `0x005E7160` | partial: modal helper refreshes/highlights but app-level modal command integration is not complete | `src/ui/skirmish_shell/state.rs`, future modal action routing in `src/app.rs` | Treat successful Create Random Map as a selection/accept path only when the native dialog returns success; canceled generation should preserve prior selection | Cancel random-map dialog leaves previous selected record; accepted random-map command commits sentinel through normal selection | `skirmish_create_random_map_cancel_preserves_previous_selection` | Do not commit the sentinel when the generator dialog returns cancel/`-1`. |

## Negative Facts / Do Not Do

- Do not create/update `RandMap.Sed` when the random-map dialog returns cancel or any value other than `1`. Active in YR: No; evidence `0x005E85C1..0x005E85CE`.
- Do not create duplicate sentinel records. Active in YR: No; evidence existing-record scan/update `0x005E8636..0x005E871F`.
- Do not set sentinel min/max from a map INI. Active in YR: No; new sentinel constructor receives hardcoded min `2`, max `4`, evidence `0x005E866E..0x005E8683`.
- Do not set sentinel `Official=false`. Active in YR: No for native-created sentinel; constructor receives official flag `1`, evidence `0x005E8674..0x005E8683`.
- Do not decode a map `PreviewPack` for `RandMap.Sed`. Active in YR: No; command loads `RandMap.img`, evidence `0x005E861A..0x005E8626`.
- Do not treat `0x00598960(1, hwnd)` preview-time generation as identical to launch generation. Active in YR: Conditional difference; preview-time passes nonzero UI params and repeatedly posts paint, launch branch passes `(0,0)`, evidence `0x00596300`, `0x00598960`, sibling launch report.

## Remaining Uncertainty

- Exact `.SED` serialized byte layout remains deferred; save/load vtable calls and timing are verified.
- Exact localized text for string id `0xF5E` remains deferred; binary buffer ownership and writes are verified.
- Deep terrain formulas inside `0x00598960` remain deferred to slot 1.

## Stale Docs / Follow-up Docs

Path: `docs/research/skirmish-ui/SKIRMISH_CHOOSE_MAP_0X6B_PREVIEW_REFRESH_GHIDRA_REPORT.md`

Replace OQ-12 with:

> `[RESOLVED] OQ-12 - Create Random Map command 0x583 hides the chooser, calls 0x005E8590, and only continues when the random-map dialog returns 1. The accepted path saves DAT_00ABDFD8 seed/options to RandMap.Sed through 0x00597730, replaces DAT_00AC1154 from RandMap.img, update-or-appends exactly one RandMap.Sed scenario record, reselects that record in listbox 0x553, restores the previous committed index before the normal accept helper, and then commits through 0x005E7160. Terrain formulas inside 0x00598960 remain owned by the generator-internals report.`

Path: `docs/research/skirmish-ui/SKIRMISH_RANDMAP_SED_RANDOM_MAP_BEHAVIOR_GHIDRA_REPORT.md`

Refine the sentinel creation/update paragraph with:

> `The 0x583 command calls 0x005E8590 only after hiding the chooser. 0x005E8590 returns -1 unless the random-map dialog pump returns 1. On success it writes DAT_008316D4=1, saves DAT_00ABDFD8 to RandMap.Sed through 0x00597730, rebuilds DAT_00AC1154 from RandMap.img, and then either updates the existing RandMap.Sed record's display/digest or constructs a new official record with min players 2 and max players 4.`

## Sources

- Ghidra read-only decompile / assembly: `0x005E69FD..0x005E6B57`, `0x005E8590..0x005E871F`, `0x00595BC0`, `0x00596300`, `0x00596C70`, `0x00596E50`, `0x00597260`, `0x005975E0`, `0x00597730`, `0x00597A10`, `0x005E84D0`, `0x005E70D0`, `0x005E7BF0`, `0x005E7160`, `0x0069A980`, `0x0069ADF0`, `0x0069ACD0`, `0x0069AD80`, `0x006406E0`, `0x006406F0`, `0x00641DB0`.
- Prior docs referenced: `SKIRMISH_CHOOSE_MAP_0X6B_PREVIEW_REFRESH_GHIDRA_REPORT.md`, `SKIRMISH_RANDMAP_SED_RANDOM_MAP_BEHAVIOR_GHIDRA_REPORT.md`, `SKIRMISH_RANDOM_MAP_BRANCH_AFTER_SELECTED_MAP_LOAD_GHIDRA_REPORT.md`.
- Rust scan: `src/skirmish_scenarios.rs`, `src/ui/skirmish_shell/state.rs`, `src/app_skirmish_shell_render.rs`, `src/app_list_maps.rs`, `src/app_init.rs`, `src/app_transitions.rs`.
