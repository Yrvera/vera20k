# Skirmish Scenario Source Population Order - Ghidra Research Report

**Date:** 2026-05-22  
**Address(es):** `0x00699980`, `0x0069A3B0`, `0x0069A980`, `0x006994F0`, `0x005D63E0`, `0x0069AE10`, `0x005E6EA6..0x005E7031`, `0x005E7160`  
**Investigation Mode:** coverage-map, targeted verification of the source-population slice  
**Claimed Scope:** Offline Yuri's Revenge Skirmish scenario/map record source order feeding the Choose Map list: `MISSIONSMD.PKT [MultiMaps]`, loose `*.PKT`, loose `*.YRO` plus its embedded `MISSIONS.YRO`/PKT path, loose `*.YRM`, record append/duplicate behavior, and current Rust deltas.  
**Non-Scope:** PreviewPack decoding internals, random terrain generation, full modal visual layout, exact localized CSF strings, runtime filesystem ordering beyond Win32 API contract, and post-selection file-open asset override priority.  
**Confidence:** High for active source branch order, no-sort/no-dedupe append behavior, record field construction, modal consumption order, and Rust mismatch; Medium for exact loose-file order because `FindFirstFileA`/`FindNextFileA` returns filesystem order.  
**Active in YR:** Yes. The source builder uses YR-specific strings `MISSIONSMD.PKT`, `*.YRO`, and `*.YRM` in the live scenario-list builder consumed by the standard offline Skirmish Choose Map modal.

## 1. Overview

The Choose Map modal does not scan map files when opened. It consumes the global scenario record pointer array `DAT_00A8B8CC[0..DAT_00A8B8D8)`, filters that array by the selected `MPModesMD` mode/category, and preserves the record array's original append order.

For standard YR, the active source builder appends records in this order: stock/virtual `MISSIONSMD.PKT [MultiMaps]`, loose `*.PKT [MultiMaps]`, loose `*.YRO` records driven through `MISSIONS.YRO`/embedded PKT data, then loose `*.YRM` direct records. No duplicate-name or duplicate-path replacement was found in this builder; duplicate records remain separate pointer identities in the chooser.

## 2. Key Offsets / Globals

| Item | Meaning | Evidence | Active in YR |
|---|---|---|---|
| `DAT_00A8B8CC` | global scenario-record pointer array consumed by modal | `0x005E6F17`, `0x005E7160`; prior list report | Yes |
| `DAT_00A8B8D8` | global scenario-record count | `0x005E6EF6`, `0x005E7160`; prior list report | Yes |
| caller object `+0x690` | dynamic vector/list cleared and appended by source builder | `0x0069999F..0x006999D6`, append blocks through `param_1+0x694/+0x6A0` | Yes |
| record size `0x1BC` | allocated for each scenario record | `operator_new(0x1BC)` at `0x00699A57`, `0x00699BA8`, `0x00699E38`, `0x0069A101` | Yes |
| record `+0x00` | wide display title used by chooser/static text | `0x0069A3B0`, YRO suffix rewrite `0x00699E6B..0x00699F1F`, `0x0069A980` | Yes |
| record `+0x58` | ASCII map file/path token used later for selected-map load | `0x0069A3B0`, `0x0069A980`; selected loader reports | Yes |
| record `+0x17C` | `[Basic] Official` flag; optional filter can reject unofficial records | `0x006994F0`, `0x005D63E0` | Conditional: active only when `FUN_0077D940()` mode gate is true |
| record `+0x180/+0x184` | `[Basic] MinPlayers/MaxPlayers`; YRO suffix and map capacity use these | `0x006994F0`, `0x00699E6B`, record decode report | Yes |
| record `+0x1B4` | count of parsed map `GameModes` entries | `0x0069AE10` | Yes |
| control `0x553` | Choose Map map list item data is record pointer | `0x005E7160`; prior modal reports | Yes |
| control `0x6EB` | selected mode/category item data is mode pointer | `0x005E7160`; MPModes report | Yes |

## 3. Source Collection Order

### 3.1 `MISSIONSMD.PKT [MultiMaps]` first

`0x00699980` clears/reuses the scenario list, constructs a file object for string `0x0083F520 = "MISSIONSMD.PKT"`, and opens it through the game's file system path. If open succeeds, it reads section `0x0083F514 = "MultiMaps"`, gets the entry count, then iterates entries by increasing numeric index.

Each non-empty entry value is read into a `0x40` byte stack buffer and constructs one `0x1BC` record through `0x0069A3B0`. The append block immediately stores the returned record pointer at `*(param_1+0x694) + count*4` and increments `param_1+0x6A0`.

Evidence:

- `0x006999D9` pushes `MISSIONSMD.PKT`; `0x006999E2..0x006999F3` opens/tests the file.
- `0x00699A08` pushes `MultiMaps`.
- `0x00699A24..0x00699A3F` loops index reads from `MultiMaps` into a `0x40` buffer.
- `0x00699A68..0x00699A77` allocates/calls `0x0069A3B0`.
- `0x00699A7E..0x00699AB7` appends without comparing existing records.

Active in YR: Yes. This is the first source branch in the live YR builder and uses the YR-specific `MISSIONSMD.PKT` file name.

### 3.2 Loose `*.PKT` second

After the `MISSIONSMD.PKT` pass, `0x00699980` calls `FindFirstFileA("*.PKT")`, skips directory/system-like attributes by testing `dwFileAttributes & 0x116`, opens each loose PKT, and repeats the same `MultiMaps` count/index loop and `0x0069A3B0` constructor.

Evidence:

- `0x00699AE1..0x00699AEC` calls `FindFirstFileA` with `0x0083F50C = "*.PKT"`.
- `0x00699AFB..0x00699B06` rejects entries with attribute mask `0x116`.
- `0x00699BB9..0x00699BC8` allocates/calls `0x0069A3B0`.
- `0x00699BD3..0x00699C0C` appends the new pointer without checking the prior `MISSIONSMD.PKT` records.

Active in YR: Yes. It is unconditional after the stock `MISSIONSMD.PKT` pass. Loose file enumeration order inside this source group is the Win32 filesystem order, not a game sort.

### 3.3 Loose `*.YRO` / embedded PKT third

The next branch calls `FindFirstFileA("*.YRO")`. For each eligible loose YRO, it verifies/open-checks `MISSIONS.YRO`, constructs/opens the embedded PKT path by replacing the `.YRO` suffix with `PKT`, reads that embedded PKT's `MultiMaps`, constructs records through `0x0069A3B0`, and then rewrites the display title with a player-count suffix before append.

Evidence:

- `0x00699C58..0x00699C5D` calls `FindFirstFileA` with `0x0083F504 = "*.YRO"`.
- `0x00699CB8..0x00699CC8` pushes/checks `0x0083F4F4 = "MISSIONS.YRO"`.
- `0x00699D4C..0x00699DA8` is the embedded PKT-name construction path using suffix string `0x0083F4F0 = "PKT"`.
- `0x00699E4D..0x00699E5C` constructs each embedded-PKT record with `0x0069A3B0`.
- `0x00699E6B..0x00699F1F` appends ` (n)` or ` (min-max)` to the wide display title, bounded to `0x2C` wide slots and terminated at `record+0x56`.

Active in YR: Yes for loose YRO archives. This branch is before loose `*.YRM`, so YRO-derived records precede YRM custom maps in the chooser after filtering.

### 3.4 Loose `*.YRM` fourth

The final map-source branch calls `FindFirstFileA("*.YRM")`. Each eligible loose YRM is read directly for `[Basic] Name`, `[Digest]`, `[Basic] Official`, player limits, and game-mode data; it then constructs a direct record through `0x0069A980`.

Evidence:

- `0x0069A002..0x0069A007` calls `FindFirstFileA` with `0x0083F490 = "*.YRM"`.
- `0x0069A024..0x0069A02F` applies the same `dwFileAttributes & 0x116` reject mask.
- `0x0069A056..0x0069A073` reads `[Basic] Name`, defaulting to `No Name`.
- `0x0069A0FC` calls `0x006994F0` to read first/last-file metadata including `MinPlayers`, `MaxPlayers`, `GameMode`, `Digest`, and `Official`.
- `0x0069A112..0x0069A13C` calls `0x0069A980`, which writes record `+0x58`, `+0x00`, `+0x15C`, `+0x17C`, `+0x180`, and `+0x184`.

Active in YR: Yes for loose YRM files. This builder did not enumerate loose `.MMX`, `.MPR`, or arbitrary `.MAP` in the active standard YR source-population branch.

## 4. Duplicate / Override Behavior

No source branch performs duplicate suppression by display title, file/path token, digest, or record contents before appending to the scenario list. The observed append pattern is count/capacity/grow check, store pointer at current count, increment count. There is no call in the branch to compare the new record against existing `DAT_00A8B8CC`/caller-vector entries.

Verified implications:

- A loose `*.PKT` record with the same `MultiMaps` entry as `MISSIONSMD.PKT` does not replace the stock record at list-construction time. Both records are retained if both constructors succeed.
- A loose YRO-derived record and a loose YRM record with matching display names or path-like names are separate records if both append.
- The chooser stores each row's scenario-record pointer as item data. Duplicate labels are therefore still distinguishable internally by pointer, and accept scans the global array for that pointer.
- User/loose sources appear after stock/virtual `MISSIONSMD.PKT`, not before it. There is no stock-vs-user override in the chooser record array itself.

Active in YR: Yes. Evidence: append blocks in `0x00699A7E..0x00699AB7`, `0x00699BD3..0x00699C0C`, `0x00699F2B..0x00699F70`, and `0x0069A147..0x0069A17E`; pointer commit in `0x005E7160`.

Important boundary: this finding is about records feeding the chooser list. Later selected-map loading opens the selected record's `+0x58` path through the file system; whether a loose map payload with the same file name shadows an archive payload during that later open is a separate asset-resolution question and is not claimed here.

## 5. Record Field Construction Details

PKT-style records from `MISSIONSMD.PKT`, loose `*.PKT`, and embedded-YRO PKT all use `0x0069A3B0`:

- record `+0x58` is the `MultiMaps` entry value with `.MAP` appended. Active in YR: Yes; evidence `0x0069A46A..0x0069A4BF`, suffix `0x0082DF18 = ".MAP"`.
- display title first uses `DescriptionText`, else localized `Description`. Active in YR: Yes; evidence `0x0069A4DE..0x0069A53A` and `SKIRMISH_CHOOSE_MAP_YRO_DISPLAY_STRING_CONSTRUCTION_GHIDRA_REPORT.md`.
- `Official` defaults to `1` before map metadata reads. Active in YR: Yes; evidence `0x0069A3B0` writes byte `record+0x17C = 1`.
- `MinPlayers/MaxPlayers` default to `2/4` before reads. Active in YR: Yes; evidence `0x0069A3B0`.
- digest defaults to `No Digest`, bounded by terminator at `record+0x17B`. Active in YR: Yes; evidence `0x0069A3B0`.

Direct loose `*.YRM` records use `0x0069A980`:

- null file/path writes `No File Name`; normal path uses `_strncpy(record+0x58, ..., 0x104)` and forces `record+0x15B = 0`. Active in YR: Conditional for null, Yes for normal YRM; evidence `0x0069A980`.
- display title uses passed `[Basic] Name` converted to wide, bounded to `0x2C` wide slots, with terminator at `record+0x56`; null title loads string-table id `0xB1D`. Active in YR: Yes for non-null live YRM branch, Conditional for null fallback.
- digest uses passed digest or `No Digest`, bounded to `0x20` bytes plus `record+0x17B = 0`. Active in YR: Yes.
- direct YRM constructor inserts a source/CD list value `0xFFFFFFFE` into the record's source list before optional GameModes parsing. Active in YR: Yes; evidence `0x0069A980` writes `0xfffffffe`.

## 6. Modal Consumption / Order Preservation

The modal population path starts at custom-message handling around `0x005E6EA6`. It finds the selected mode object, populates mode combo/list `0x6EB`, allocates a temporary pointer vector sized from `DAT_00A8B8D8`, then loops the global record array from index `0` upward.

For each record, it calls `0x005D63E0(selected_mode, record)`. Accepted record pointers are appended into the temporary vector through `0x005EEE40`; then that vector is passed to the listbox backing object for control `0x553`.

Evidence:

- `0x005E6EA6..0x005E6EC7` resolves selected mode/category from `DAT_00A8B250`.
- `0x005E6EF6` reads `DAT_00A8B8D8`.
- `0x005E6F17..0x005E6F45` loops `EBX = 0..count-1`, loads `DAT_00A8B8CC[EBX]`, calls `0x005D63E0`, and appends if true.
- `0x005E6F47..0x005E6F5B` installs the temporary vector into control `0x553`.
- No compare/sort function exists in this loop.

Active in YR: Yes. This is the live Choose Map modal initialization path. The player-visible order is therefore source append order, only filtered by the selected mode/category.

## 7. Filter Predicate

`0x005D63E0` applies the selected-mode filter:

1. If the record is `RandMap.Sed`, it returns the selected mode object's vtable `+0x3C` result. Active in YR: Conditional on the random-map sentinel.
2. If record `+0x17C == 0` and `FUN_0077D940()` returns true, it rejects the record. Active in YR: Conditional on that runtime mode gate.
3. Otherwise it calls `0x0069AE10` to match the selected mode filter string/list against the record's `GameModes`.

`0x0069AE10` behavior:

- If the record's `GameModes` count at `+0x1B4` is zero, it only matches selected mode string `standard`. Active in YR: Yes.
- If entries exist, it compares the selected mode string against each map `GameModes` entry in insertion order and accepts on the first match. Active in YR: Yes.

Evidence: `0x005D63E0` decompile; `0x0069AE10` decompile; `0x0083F668 = "standard"`.

## 8. Current Rust Implementation Status

Current Rust does not model the verified source-population contract.

| Area | Current Rust behavior | Delta |
|---|---|---|
| map source collection | `src/app_list_maps.rs` scans only the RA2 directory via `std::fs::read_dir` | Missing `MISSIONSMD.PKT`, loose `*.PKT MultiMaps`, embedded-YRO PKT, and exact YRM-only direct branch |
| extensions | includes loose `mmx`, `yro`, `map`, `mpr`, `yrm` | Retail source builder for this path uses `*.PKT`, `*.YRO`, `*.YRM`; no loose `.mmx/.mpr/.map` scan in this function |
| order | sorts by lowercase display name | Retail preserves source append order, with only selected-mode filtering |
| duplicate handling | sorting and path/file identity may collapse later depending implementation choices, but no retail pointer identity exists | Need record identity/source ordinal; do not replace by display/path |
| source identity | `MapMenuEntry` has file/display/preview fields only | Missing source kind, source ordinal, PKT section/stem, map filter list, official/min/max fields |
| filtering | no `MPModesMD` selected-mode filter in map list | Need `standard` default behavior and explicit `GameModes` matching |
| Choose Map | current action previously cycled selected map in-place | Need modal over filtered source-order records with pointer/id-like item data |

## 9. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `MISSIONSMD.PKT` branch | verified | `0x006999D9`, `0x00699A08`, `0x00699A68..0x00699AB7` | none for source order |
| loose `*.PKT` branch | verified | `0x00699AE1`, `0x00699BB9..0x00699C0C` | exact filesystem order requires runtime observation |
| loose `*.YRO` / embedded-PKT branch | verified | `0x00699C58`, `0x00699CB8`, `0x00699D4C..0x00699DA8`, `0x00699E4D..0x00699F70` | exact YRO archive internals beyond source record creation out of scope |
| loose `*.YRM` branch | verified | `0x0069A002`, `0x0069A056..0x0069A13C`, `0x0069A147..0x0069A17E` | none for source order |
| duplicate/override behavior | verified | append blocks; no comparison before store; pointer commit in `0x005E7160` | post-selection file-open shadowing deferred |
| modal order preservation | verified | `0x005E6F17..0x005E6F45`, `0x005EEE40` append | none |
| selected-mode filter | verified | `0x005D63E0`, `0x0069AE10` | exact category roster owned by MPModes report |
| standard YR base `MISSIONS.PKT` source | verified negative for this function | string search found no `MISSIONS.PKT`; `0x00699980` literal source is `MISSIONSMD.PKT` | separate non-YR/base executable path not claimed |
| current Rust source parity | verified gap | `src/app_list_maps.rs`, `src/app_init.rs` | implementation needed |

## 10. Open Questions - Final State

- `[RESOLVED] OQ-01 - What is the first source? -> `MISSIONSMD.PKT [MultiMaps]`, opened before loose file enumeration.` (evidence: `0x006999D9`, `0x00699A08`)
- `[RESOLVED] OQ-02 - Are loose `*.PKT` files before or after stock `MISSIONSMD.PKT`? -> After; the `FindFirstFileA("*.PKT")` branch starts only after the stock pass.` (evidence: `0x00699AE1`)
- `[RESOLVED] OQ-03 - Are loose `*.YRO` records before loose `*.YRM` records? -> Yes; `*.YRO` branch starts at `0x00699C58`, `*.YRM` starts at `0x0069A002`.` (evidence: addresses listed)
- `[RESOLVED] OQ-04 - Does the builder scan loose `.mmx`, `.mpr`, or arbitrary `.map` in this path? -> No such extension branch was found in `0x00699980`; active branches are `*.PKT`, `*.YRO`, and `*.YRM` plus `MISSIONSMD.PKT`.` (evidence: decompile and string search)
- `[RESOLVED] OQ-05 - Does a later source replace an earlier duplicate record? -> No record replacement/dedupe was found; every constructed record is appended by pointer if the vector accepts it.` (evidence: append blocks in all four branches)
- `[RESOLVED] OQ-06 - What preserves duplicate rows in the modal? -> The list item data is the scenario-record pointer, and accept scans `DAT_00A8B8CC` for that pointer.` (evidence: `0x005E7160`)
- `[RESOLVED] OQ-07 - Is modal display order sorted by label? -> No; modal loops global records from index 0 upward and appends accepted pointers.` (evidence: `0x005E6F17..0x005E6F45`)
- `[RESOLVED] OQ-08 - How do empty `GameModes` lists filter? -> Empty map GameModes accepts only selected mode string `standard`.` (evidence: `0x0069AE10`, string `0x0083F668`)
- `[RESOLVED] OQ-09 - Is `MISSIONS.PKT` part of this standard YR source builder? -> No literal/string source was found; the verified literal is `MISSIONSMD.PKT`.` (evidence: Ghidra string search; `0x006999D9`)
- `[DEFERRED] OQ-10 - Exact loose file enumeration order across NTFS/FAT and Wine/locales.` (category: needs-runtime-debugger; reason: binary delegates ordering to `FindFirstFileA/FindNextFileA`; next-step-if-pursued: runtime probe with multiple loose PKT/YRO/YRM files on the target filesystem)
- `[DEFERRED] OQ-11 - Whether selected file open later lets a loose `.map` shadow an archive map with the same `+0x58` token.` (category: requires-different-system-context; reason: this slot is source-population, not selected-map file-open priority; next-step-if-pursued: investigate `0x005E7BF0`/file system open resolution for duplicate names)
- `[DEFERRED] OQ-12 - Exact localized visible labels for every stock map.` (category: out-of-scope; reason: source order and display construction are verified, but CSF text dump is a data census task; next-step-if-pursued: asset census over `MISSIONSMD.PKT` and language tables)

## 11. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Standard YR builds chooser records from `MISSIONSMD.PKT`, loose `*.PKT`, loose `*.YRO` embedded PKT, then loose `*.YRM`, preserving append order. | `0x00699980`; strings `0x0083F520`, `0x0083F50C`, `0x0083F504`, `0x0083F490` | missing; Rust scans loose map extensions and sorts by name | `src/app_list_maps.rs`, `src/app_init.rs::MapMenuEntry`, future chooser model | Build a source-ordered scenario-record list with explicit source kind and ordinal before filtering. | `skirmish_map_sources_preserve_retail_order_missionsmd_pkt_before_loose_sources`: fixture records appear in stock-PKT, loose-PKT, YRO, YRM order. | Do not sort Choose Map records by display name. |
| Later source records do not replace earlier duplicate display/path records; row identity is record pointer/item data. | append blocks in `0x00699A7E`, `0x00699BD3`, `0x00699F2B`, `0x0069A147`; `0x005E7160` pointer scan | missing pointer-like stable identity | chooser state and selection commit path | Preserve duplicates as distinct records with stable ids; selected row commits by id/source ordinal, not display text. | `skirmish_duplicate_map_records_are_retained_and_selectable_by_identity`: two fixture records with same label both appear and commit distinct ids. | Do not use display name or file path as a unique map key in the chooser. |
| Empty map `GameModes` matches only selected mode string `standard`; explicit lists match selected mode entries. | `0x0069AE10`; `0x005D63E0` | missing; no selected-mode filter | future MPModes/map filter model | Filter the source-ordered list after selected mode changes, with standard fallback for empty mode lists. | `skirmish_choose_map_empty_gamemodes_visible_only_for_standard`: empty GameModes appears in Battle/FreeForAll standard but not TeamGame. | Do not treat empty GameModes as "all modes". |
| The standard YR builder in this slice does not enumerate loose `.mmx`, `.mpr`, or arbitrary `.map`; direct loose custom records are `*.YRM`. | `0x00699980` decompile/string search | Rust includes `mmx`, `map`, `mpr` in menu list | `src/app_list_maps.rs` | Separate dev/main-menu loose-map convenience from retail Skirmish Choose Map source list, or gate non-retail extensions explicitly. | `skirmish_retail_source_scan_ignores_loose_mmx_mpr_map_for_choose_map`: fixture directory with `.mmx/.mpr/.map/.yrm` yields only `.yrm` direct record for the retail chooser. | Do not feed the retail Skirmish modal from the current broad loose-file scanner. |

## Negative Facts / Do Not Do

- Do not implement the retail Skirmish Choose Map list by sorting `MapMenuEntry.display_name`.
- Do not collapse records by display name, map stem, or path; native uses record pointers and retains duplicates.
- Do not treat loose user maps as overriding stock `MISSIONSMD.PKT` records in the chooser list itself; they append later.
- Do not claim `MISSIONS.PKT` is part of this standard YR `0x00699980` source path; no literal/source branch was found in this binary function.
- Do not use `.mmx`, `.mpr`, or arbitrary `.map` loose-file scans for the retail Skirmish modal unless a separate active YR path is verified.
- Do not reject maps with empty `GameModes` universally; empty means `standard` only.

## Remaining Uncertainty

- The exact order returned by `FindFirstFileA/FindNextFileA` for loose `*.PKT`, `*.YRO`, and `*.YRM` is filesystem/runtime order. The binary does not add a game-level sort in this path.
- Later selected-file opening may shadow archive payloads with loose files of the same `record+0x58` token; that is outside this source-population report.
- Exact localized stock labels require CSF/language-resource census, not more control-flow work.

## Stale Docs / Follow-up Docs

- In `SKIRMISH_MIX_ARCHIVE_MAP_HEADER_CENSUS_GHIDRA_REPORT.md`, replace wording that implies the same live YR source builder consumes `MISSIONSMD.PKT / MISSIONS.PKT` with:

  > The standard YR source-population builder verified at `0x00699980` opens `MISSIONSMD.PKT` first. This report did not verify a `MISSIONS.PKT` literal/source branch in that same YR builder; base RA2 `MISSIONS.PKT` asset census findings should be treated as a separate/base-data audit unless a separate active YR path is proven.

- In `SKIRMISH_CHOOSE_MAP_LIST_POPULATION_ORDER_GHIDRA_REPORT.md`, strengthen the loose-source caveat with:

  > The binary preserves source append order and delegates ordering within each loose file group to `FindFirstFileA/FindNextFileA`; it performs no display-name sort and no duplicate suppression before the modal filter.

## Sources

- Ghidra read-only decompile: `0x00699980`, `0x0069A3B0`, `0x0069A980`, `0x006994F0`, `0x005D63E0`, `0x0069AE10`, `0x005E7160`.
- Ghidra read-only assembly context: `0x006999D9`, `0x00699A08`, `0x00699A68`, `0x00699AE1`, `0x00699BB9`, `0x00699C58`, `0x00699CB8`, `0x00699D4C`, `0x00699E4D`, `0x00699E6B`, `0x0069A002`, `0x0069A056`, `0x0069A13C`, `0x005E6EA6`, `0x005E6F17`, `0x005E6F45`.
- Ghidra string evidence: `0x0083F520 = "MISSIONSMD.PKT"`, `0x0083F514 = "MultiMaps"`, `0x0083F50C = "*.PKT"`, `0x0083F504 = "*.YRO"`, `0x0083F4F4 = "MISSIONS.YRO"`, `0x0083F4F0 = "PKT"`, `0x0083F490 = "*.YRM"`, `0x0083F668 = "standard"`.
- Prior docs read: `SKIRMISH_CHOOSE_MAP_LIST_POPULATION_ORDER_GHIDRA_REPORT.md`, `SKIRMISH_CHOOSE_MAP_RECORD_DECODE_FIELDS_GHIDRA_REPORT.md`, `SKIRMISH_CHOOSE_MAP_YRO_DISPLAY_STRING_CONSTRUCTION_GHIDRA_REPORT.md`, `SKIRMISH_MIX_ARCHIVE_MAP_HEADER_CENSUS_GHIDRA_REPORT.md`, `SKIRMISH_MPMODES_OBJECT_CONSTRUCTION_DEFAULTS_GHIDRA_REPORT.md`.
- Rust scan: `src/app_list_maps.rs`, `src/app_init.rs`, `src/ui/skirmish_shell/state.rs`.
