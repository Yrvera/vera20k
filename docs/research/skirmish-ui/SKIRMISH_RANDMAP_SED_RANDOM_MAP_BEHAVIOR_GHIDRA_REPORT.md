# Skirmish RandMap.Sed Random Map Behavior - Ghidra Research Report

**Address(es):** `0x005E6920/LAB_005E6920`, `0x005E8590`, `0x005D63E0`, `0x005D6350`, `0x005E7160`, `0x005E7BF0`, `0x00684620`, `0x00596300`, `0x00598960`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** offline Skirmish Choose Map behavior for the `RandMap.Sed` random-map sentinel: list membership, selected-mode filtering, accept commit token, preview branch, and launch resolution.  
**Non-Scope:** exact random terrain generation algorithms, random-map dialog visual layout, full random-map seed object fields, online/WOL variants, and exact localized CSF text for the random-map display name.  
**Confidence:** High for sentinel creation/filter/commit/launch branch; Medium for display-string human text because this slice verifies the buffer and string-table source, not the resolved CSF text.  
**Active in YR:** Yes / Conditional. The code is live in YR offline Skirmish Choose Map; random-map behavior is conditional on using the modal random-map command and on selected mode allowing random maps.

## 1. Overview

`RandMap.Sed` is a synthetic scenario-record filename used as the random-map sentinel. It is not produced by loose map scanning; the Choose Map dialog command `0x583` calls `0x005E8590`, which opens/uses the random-map generator dialog and then creates or updates a `DAT_00A8B8CC` record whose file token is `RandMap.Sed`.

Once that record exists, it participates in the normal Choose Map list as scenario-record item data. The list admits it only when the selected `MPModesMD.ini` mode's random-map flag is true. Accepting it commits the ordinary selected record index; later launch resolves the filename `RandMap.Sed` through `ScenarioClass__Read_Scenario`, which detects `.SED` and runs the random-map generation path instead of the normal map INI reader.

## 2. Class Layout / Key Offsets

| Field / global | Meaning | Evidence | Active in YR |
|---|---|---|---|
| `DAT_00A8B8CC` / `DAT_00A8B8D8` | global scenario-record pointer array and count; holds the synthetic random record after creation | `0x005E8590`, `0x005D6370`, `0x005E7160` | Yes |
| record `+0x00` | display/name string sent to map text/static controls | `0x0069ACD0`, `0x0069A980`, `0x005E7160 -> 0x5A8` | Yes |
| record `+0x58` | file/path token; compared to `RandMap.Sed` | `0x0069ADF0`; string `0x0082BC30` | Conditional: true for random sentinel |
| record `+0x15C` | digest/source string; random path updates it from `0x005E84D0` | `0x005E8590`, `0x0069AD80` | Conditional |
| record `+0x17C` | official flag; synthetic record constructor passes true | `0x005E8590 -> 0x0069A980` | Conditional |
| record `+0x180/+0x184` | min/max players; synthetic record constructor passes `2` / `4` | `0x005E8590 -> 0x0069A980` | Conditional |
| mode object `+0x34` | random maps allowed byte parsed from `MPModesMD.ini` fifth column | `0x005D7590`, `0x005D5B60`, `0x005D6350` | Conditional by selected mode |
| `DAT_00ABE050` | random-map name/display buffer used when creating/updating the sentinel record | `0x005E8590`, `0x00596300` | Conditional |
| `DAT_00AC1154` | preview wrapper; random branch loads `RandMap.img` | `0x006ACEE0`, `0x006AE6E0`, `0x005E8590` | Conditional |
| `ScenarioClass+0x34BD` | `IsRandom` byte set by `.SED` filename detection | `0x00684694..0x006846BE` | Conditional |
| `ScenarioClass+0x125C` | scenario filename buffer; receives `RandMap.Sed` before launch | `0x005E7BF0`, `0x00684620` | Yes |

## 3. Core Logic

### Random-map sentinel creation/update

The Choose Map callback handles command ids around `0x005E69B7`. `0x5C0` closes with result `2`; `0x6C5` is the accept button; command `0x583` enters the random-map path. The `0x583` branch calls pre-modal shell handling, then calls `0x005E8590`. Active in YR: Conditional on pressing the random-map command in the live Choose Map dialog. Evidence: assembly `0x005E69C2..0x005E6A18`, direct call at `0x005E6A11`; direct-call scan found this as the only direct call to `0x005E8590`.

`0x005E8590` first calls `0x00595BC0`; if that returns anything other than `1`, it returns `-1` and no sentinel commit/list update occurs. If it returns `1`, it sets a byte at `0x008316D4`, calls `0x00597730("RandMap.Sed")`, rebuilds `DAT_00AC1154` from `RandMap.img`, and scans `DAT_00A8B8CC[0..DAT_00A8B8D8)`. Active in YR: Conditional. Evidence: decompile `0x005E8590`.

If an existing record's `+0x58` already equals `RandMap.Sed`, `0x005E8590` updates that record in place: it copies the current `DAT_00ABE050` random-map name buffer into the record display/name field via `0x0069ACD0`, computes a digest/source string with `0x005E84D0`, stores it via `0x0069AD80`, and returns the existing index. Active in YR: Conditional. Evidence: decompile `0x005E8590`, helper `0x0069ADF0`.

If no sentinel exists, `0x005E8590` allocates a `0x1BC` scenario record and constructs it with file token `RandMap.Sed`, display/name buffer `DAT_00ABE050`, computed digest from `0x005E84D0`, official flag true, no explicit map `GameModes` list argument, min players `2`, and max players `4`; it appends the pointer to `DAT_00A8B8CC` and increments `DAT_00A8B8D8`. Active in YR: Conditional. Evidence: decompile `0x005E8590 -> 0x0069A980`; `0x0069A980` field writes.

### List membership and filtering

The normal map-list predicate at `0x005D63E0` checks the selected record first with `0x0069ADF0`. That helper compares `record+0x58` to `RandMap.Sed` at `0x0082BC30`. If true, `0x005D63E0` returns the selected mode object's vtable slot `+0x3C` result and skips both the `Official` gate and the map `GameModes` string comparison. Active in YR: Conditional. Evidence: assembly `0x005D63E8..0x005D63FC`; `0x0069ADF0` assembly `0x0069ADF0..0x0069AE06`.

The common random-map-allowed method at `0x005D6350` returns byte `mode+0x34`. The common constructor stores the fifth `MPModesMD.ini` row field into `mode+0x34`, and `0x005D7590` parses that fifth comma token. In stock `ini/mpmodesmd.ini`, only Battle id `1` and FreeForAll id `2` have `randomMapsAllowed=true`; Team Game, Megawealth, Duel, Meat Grinder, Naval War, Unholy, and Cooperative have false. Active in YR: Yes / Conditional by mode. Evidence: assembly `0x005D6350`; decompile `0x005D5B60`; decompile `0x005D7590`; `ini/mpmodesmd.ini`.

Therefore `RandMap.Sed` can appear in the chooser list only after the sentinel record exists and only under modes whose random-map flag returns true. It is not accepted by ordinary map `GameModes` fallback and should not be treated as a `standard` map just because the synthetic record has no map-mode list.

### Accept/commit token

Accept uses the same commit path as concrete map records. `0x005E7160` reads listbox/control `0x553` current selection (`0x188`), reads item data (`0x199`), scans `DAT_00A8B8CC` for the matching record pointer, and writes `DAT_00A8B254 = matched_index`. The selected mode pointer from control `0x6EB` may also update `DAT_00A8B23C` and `DAT_00A8B250`. Active in YR: Yes. Evidence: decompile `0x005E7160`.

There is no special accept token such as a negative index for random maps. The committed token is the ordinary scenario-record index whose record path is `RandMap.Sed`. Active in YR: Yes / Conditional for sentinel. Evidence: `0x005E7160` pointer-to-index scan plus `0x005E7BF0(index)` selected-record loader.

### Preview behavior

After a random sentinel is selected or restored, Skirmish preview code does not try to decode a stock map `PreviewPack`. The setup init and Choose Map parent paths detect `RandMap.Sed` with `0x0069ADF0`, destroy/recreate `DAT_00AC1154`, and load `RandMap.img` (`0x00829ABC`). Active in YR: Conditional on selected random sentinel. Evidence: `0x006ACEE0` random branches, `0x006AE6E0` init branch, `0x005E8590` creation path, string xrefs to `RandMap.img`.

The common preview-loader helper uses `0x0069AE70`, which compares a selected-map wrapper/source field at `+0x6A8` with `RandMap.Sed`, to avoid the normal preview decode path for random maps. Active in YR: Conditional. Evidence: `0x0069AE70` assembly `0x0069AE70..0x0069AE89`; sibling preview reports agree.

### Launch resolution

`0x005E7BF0(index)` copies `DAT_00A8B8CC[index]+0x58` into `DAT_00A8B8E0`, then into `ScenarioClass+0x125C`. For a random record, that copied filename is exactly `RandMap.Sed`. Active in YR: Yes / Conditional for sentinel. Evidence: decompile `0x005E7BF0`.

`ScenarioClass__Read_Scenario @ 0x00684620` copies the filename to a local buffer, compares the filename suffix with string `.SED` at `0x0083DA88`, and sets `ScenarioClass+0x34BD = 1` on equality. For random maps it calls `0x00597A10(local_filename)`; on success it calls `0x00598960(0,0)` and `ScenarioClass__Post_Map_Init` rather than `ScenarioClass__Read_Scenario_INI`. Active in YR: Conditional on `.SED`; no TS-only gate found. Evidence: decompile `0x00684620`; assembly `0x0068465C..0x006846BE`; PE byte read confirms `0x0083DA88` is `.SED`.

## 4. INI Keys

| File / section / key | Value / meaning | Evidence | Active in YR |
|---|---|---|---|
| `ini/mpmodesmd.ini:[Battle] 1` | `..., standard, true`; Battle allows random maps | local INI; `0x005D7590`; `0x005D5B60`; `0x005D6350` | Yes |
| `ini/mpmodesmd.ini:[FreeForAll] 2` | `..., standard, true`; FreeForAll allows random maps | same | Conditional by selected mode |
| `ini/mpmodesmd.ini` ids `3,4,5,6,7,8,9` | fifth field `false`; random sentinel filtered out | same | Conditional by selected mode |
| map `[Basic] MinPlayers/MaxPlayers` | not read from a real map for sentinel; synthetic record passes min `2`, max `4` | `0x005E8590 -> 0x0069A980` | Conditional |
| map `GameModes` | not used for `RandMap.Sed`; predicate returns random flag before `0x0069AE10` | `0x005D63E0` | Conditional |

## 5. Integration Points

| Integration | Behavior | Evidence | Active in YR |
|---|---|---|---|
| Choose Map random command | control id `0x583` invokes `0x005E8590`; `-1` result skips selection/list update | `0x005E69C2..0x005E6A1F` | Conditional |
| Modal list predicate | sentinel uses selected mode random flag instead of map `GameModes` | `0x005D63E0`, `0x005D6350` | Conditional |
| Accept commit | sentinel commits as ordinary `DAT_00A8B254` index | `0x005E7160` | Yes / Conditional |
| Preview | sentinel branch loads `RandMap.img`; normal map preview path skipped | `0x006ACEE0`, `0x006AE6E0`, `0x0069AE70` | Conditional |
| Scenario load | `.SED` suffix sets `IsRandom` and routes to random generator | `0x00684620`, `0x00597A10`, `0x00598960` | Conditional |

## 6. Current Rust Implementation Status

Rust has no random-map sentinel model.

| Area | Current Rust status | Evidence |
|---|---|---|
| map record model | `MapMenuEntry` has concrete file/display/preview/start metadata only; no source kind, random sentinel, mode filter, random flag, or min/max fields | `src/app_init.rs:218..233` |
| map discovery | scans loose files by extension and sorts by display name; no `MISSIONSMD.PKT`, no synthetic `RandMap.Sed`, no `MPModesMD.ini` random flag | `src/app_list_maps.rs:23..47` |
| shell selection | stores only `selected_map_idx`; `ChooseMap` currently cycles index in place | `src/ui/skirmish_shell/state.rs:303..324`, `src/ui/skirmish_shell/state.rs:965..969` |
| launch | current selected map resolves to `available_maps[index].file_name`; no `.SED` random-map generation branch | prior Rust scan in selected-map token report; no `RandMap` matches under `src/` |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `RandMap.Sed` string xrefs | verified | string anchor report `0x0082BC30` | none |
| Choose Map command id `0x583` path | verified | assembly `0x005E69C2..0x005E6A18` | exact button caption/geometry out of scope |
| `0x005E8590` sentinel creation/update | verified | decompile `0x005E8590`, direct-call scan | random-map dialog internals out of scope |
| selected mode random flag | verified | `0x005D7590`, `0x005D5B60`, `0x005D6350`, `ini/mpmodesmd.ini` | exact vtable table dump not repeated in this report |
| modal filter short-circuit | verified | `0x005D63E0`, `0x0069ADF0` | none |
| accept commit item-data/index | verified | `0x005E7160` | none |
| preview branch | verified | `0x006ACEE0`, `0x006AE6E0`, `0x0069AE70` | exact visual contents of `RandMap.img` out of scope |
| launch `.SED` detection | verified | `0x00684620`; PE read of `0x0083DA88` string `.SED` | exact RMG generation formulas out of scope |
| current Rust delta | verified | scoped `rg` and file reads | none for status |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-RM-01 - Is RandMap.Sed a loose map scan result? -> No; it is created/updated by the Choose Map random command path at 0x005E8590.` (evidence: `0x005E6A11`, `0x005E8590`; direct-call scan)
- `[RESOLVED] OQ-RM-02 - What chooser command creates/updates it? -> command id 0x583 in the Choose Map callback.` (evidence: `0x005E69C2..0x005E6A18`)
- `[RESOLVED] OQ-RM-03 - Does it appear in the normal map list? -> Yes, once a sentinel record exists and the selected mode's random-map flag admits it; it is a normal record pointer in list item data.` (evidence: `0x005D63E0`, `0x005E7160`)
- `[RESOLVED] OQ-RM-04 - Does GameModes filtering apply to RandMap.Sed? -> No; the predicate returns selected mode vtable +0x3C before the ordinary GameModes comparison.` (evidence: `0x005D63E8..0x005D63FC`)
- `[RESOLVED] OQ-RM-05 - What data controls the vtable +0x3C result? -> common method 0x005D6350 returns mode byte +0x34, parsed from MPModesMD.ini fifth column.` (evidence: `0x005D6350`, `0x005D5B60`, `0x005D7590`, `ini/mpmodesmd.ini`)
- `[RESOLVED] OQ-RM-06 - Which stock modes allow the sentinel? -> Battle id 1 and FreeForAll id 2; other exposed stock modes have randomMapsAllowed=false.` (evidence: `ini/mpmodesmd.ini`; binary reader above)
- `[RESOLVED] OQ-RM-07 - What gets committed on accept? -> ordinary `DAT_00A8B254` index for the sentinel record; no negative random token.` (evidence: `0x005E7160`)
- `[RESOLVED] OQ-RM-08 - What file token is loaded after accept/start? -> `RandMap.Sed` is copied from record +0x58 into `DAT_00A8B8E0` and `ScenarioClass+0x125C`.` (evidence: `0x005E7BF0`)
- `[RESOLVED] OQ-RM-09 - How does launch know it is random? -> `ScenarioClass__Read_Scenario` compares the filename suffix with `.SED` and sets `ScenarioClass+0x34BD=1`.` (evidence: `0x0068465C..0x006846BE`, PE byte read at `0x0083DA88`)
- `[RESOLVED] OQ-RM-10 - What preview does the setup shell use? -> random selection loads `RandMap.img` through the preview wrapper instead of normal map PreviewPack decode.` (evidence: `0x006ACEE0`, `0x006AE6E0`, `0x005E8590`, `0x0069AE70`)
- `[RESOLVED] OQ-RM-11 - What are min/max players for the synthetic record? -> constructor arguments set min `2`, max `4`.` (evidence: `0x005E8590 -> 0x0069A980`)
- `[DEFERRED] OQ-RM-12 - What exact localized English text appears in `DAT_00ABE050` by default?` (category: bounded-cost-too-high; reason: this slice verified the string-table load and buffer use but did not decode CSF id `0xF5E`; next-step-if-pursued: CSF lookup for MapGen string id `0xF5E`)
- `[DEFERRED] OQ-RM-13 - Exact random terrain generation formulas and seed fields.` (category: out-of-scope; reason: target is chooser/launch contract; next-step-if-pursued: dedicated random-map-generator report over `0x00598960` and callees)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| `RandMap.Sed` is a synthetic scenario record created/updated by the chooser random command, not a loose file. | `0x005E6A11`, `0x005E8590` | missing | future chooser modal state; `src/app_list_maps.rs`; `src/app_init.rs` map record model | Add an explicit random-sentinel record kind/source, with file token `RandMap.Sed`, display name from random-map state, min/max `2/4`, and preview source `RandMap.img`. | `skirmish_random_map_command_adds_or_updates_single_sentinel_record`: pressing random generation twice yields one record whose display/digest update, not two duplicates. | Do not create a fake loose `RandMap.Sed` map during directory scan. |
| The sentinel is list-filtered by selected mode random flag, not map `GameModes`. | `0x005D63E0`, `0x005D6350`, `ini/mpmodesmd.ini` | missing | MPModes model; Choose Map list filtering | Include sentinel only when selected mode has `randomMapsAllowed=true`; stock Battle and FFA allow it, Team Game/ManBattle/Unholy/Coop do not. | `skirmish_choose_map_filters_randmap_by_mode_random_allowed`: Battle shows the sentinel after creation; Team Game hides it. | Do not treat absent `GameModes` as `standard` for `RandMap.Sed`. |
| Accept commits the ordinary scenario-record index; launch path opens `RandMap.Sed` and `.SED` detection runs RMG. | `0x005E7160`, `0x005E7BF0`, `0x00684620` | missing | launch session map identity; loading/scenario init | Preserve selected record identity and route `.sed`/`RandMap.Sed` to a random-map generation path instead of normal map INI loading. | `skirmish_launch_randmap_sed_uses_random_generation_path`: accepting random sentinel then Start does not attempt to parse `RandMap.Sed` as a stock map file. | Do not encode random map as `selected_map_idx = None` or as a display-name-only selection. |
| Random-map preview uses `RandMap.img`, not decoded map PreviewPack. | `0x006ACEE0`, `0x006AE6E0`, `0x0069AE70` | missing | preview texture/cache path in `src/app_skirmish_shell_render.rs` | Add a random-preview asset path/placeholder branch keyed by record kind/sentinel, and invalidate it like other accepted map changes. | `skirmish_randmap_preview_uses_randmap_img`: after selecting random sentinel, preview cache key is random sentinel and does not read `[PreviewPack]`. | Do not show the previous concrete map's thumbnail after accepting `RandMap.Sed`. |

## Negative Facts / Do Not Do

- Do not make `RandMap.Sed` a permanent loose-map scan result.
- Do not sort or de-duplicate it by display name; the native record is appended/updated in `DAT_00A8B8CC` and list item data is the record pointer.
- Do not apply ordinary map `GameModes` matching to the sentinel; the selected mode random-map flag is the gate.
- Do not commit a negative or special index on accept; commit the sentinel record index and keep its file token `RandMap.Sed`.
- Do not decode map `PreviewPack` for the sentinel; use the `RandMap.img` preview branch.
- Do not attempt to parse `RandMap.Sed` as a normal map INI at launch; `.SED` detection routes to random map generation.

## Remaining Uncertainty

- The exact localized default display text in `DAT_00ABE050` was not decoded. The binary path and buffer ownership are verified; the human text needs a CSF lookup.
- The random-map generation algorithm after `0x00598960` was intentionally not drained. This report verifies that launch reaches that path, not how terrain is generated.
- The random-map dialog's visual controls and button caption for command `0x583` are outside this slice.

## Stale Docs / Follow-up Docs

- Refine prior wording that says "`RandMap.Sed` special path" to:
  > `RandMap.Sed` is a synthetic scenario-record filename created/updated by the Choose Map random-map command (`0x583 -> 0x005E8590`). Once present, it is filtered into the chooser only when the selected MPModes object's random-map flag (`mode+0x34`, roster fifth column) is true. Accept commits the ordinary record index; launch detects the `.SED` filename suffix and runs the random-map generation path.
- Refine any "zero GameModes means standard" summaries with:
  > The empty-`GameModes` fallback applies to ordinary records only. `RandMap.Sed` short-circuits before `GameModes` comparison.

## Sources

- Ghidra read-only decompile / assembly: `0x005E68A0`, `LAB_005E6920` / `0x005E69C2..0x005E6A18`, `0x005E8590`, `0x00595BC0`, `0x00597730`, `0x00596300`, `0x00598960`, `0x005D63E0`, `0x005D6350`, `0x005D5B60`, `0x005D7590`, `0x0069ADF0`, `0x0069AE70`, `0x0069A980`, `0x0069ACD0`, `0x0069AD80`, `0x005E7160`, `0x005E7BF0`, `0x00684620`, `0x00597A10`.
- String/data evidence: `0x0082BC30` `RandMap.Sed`, `0x0082BB44` `RandMap.Map`, `0x00829ABC` `RandMap.img`, `0x0083DA88` `.SED`.
- Local data checked: `C:/Users/enok/Documents/ra2-rust-game/ini/mpmodesmd.ini`.
- Prior docs referenced: `SKIRMISH_CHOOSE_MAP_LIST_POPULATION_ORDER_GHIDRA_REPORT.md`, `SKIRMISH_SELECTED_MAP_TOKEN_LOAD_CONSUMER_GHIDRA_REPORT.md`, `SKIRMISH_MPMODES_OBJECT_CONSTRUCTION_DEFAULTS_GHIDRA_REPORT.md`, `SKIRMISH_CHOOSE_MAP_PREVIEW_INVALIDATION_GHIDRA_REPORT.md`, `SKIRMISH_PREVIEW_OBJECT_LIFECYCLE_DAT_00AC1154_GHIDRA_REPORT.md`.
- Rust scan: `src/app_init.rs`, `src/app_list_maps.rs`, `src/ui/skirmish_shell/state.rs`, `src/app.rs`, `src/app_skirmish_shell_render.rs`.
