# Skirmish Selected Map File-Open Priority / Loose Shadowing - Ghidra Research Report

**Address(es):** `0x005E7BF0`, `0x00683AB0`, `0x00686730`, `0x004739F0`, `0x0047AE10`, `0x00473C50`, `0x00473D10`, `0x005B4430`, `0x00431F10`, `0x0065CB50`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** Standard offline Skirmish selected record `+0x58` file-token resolution after Choose Map/Start, specifically whether a loose filesystem map can shadow an archive-contained map with the same token and the exact read/open priority used by the scenario INI path.  
**Non-Scope:** Scenario source population order, duplicate row append order, Choose Map visuals, preview decoding, MIX archive load order census, random `.SED` generation internals, online/WOL launch variants, and runtime experiments with file insertion during shell/load.  
**Confidence:** High for standard offline Skirmish path, availability-vs-read distinction, and loose-before-MIX scenario parse priority; Medium for exact loaded-MIX linked-list order when no loose file exists because that list is populated outside this slice.  
**Active in YR:** Yes. Standard offline Skirmish copies selected record `+0x58` through `0x005E7BF0` into `ScenarioClass+0x125C`, then `Main_Game` calls `ScenarioClass__Start_Scenario @ 0x00683AB0`, which reaches `ScenarioClass__Read_Scenario_INI @ 0x00686730` for ordinary non-`.SED` maps.

## Working Notes

- Target question: Does the later selected Skirmish map file open let a loose `.MAP` shadow an archive map with the same selected record `+0x58` token/path?
- Non-goals: Do not re-investigate scenario record source append order, modal accept/cancel behavior, selected-token handoff, preview renderer, or random terrain formulas.
- Evidence needed to mark COMPLETE: selected-record loader open/copy evidence, post-shell scenario open evidence, `CCFileClass` set-name evidence, availability check evidence, actual read/open priority evidence, MIX lookup evidence, raw filesystem probe/open evidence, and current Rust surface scan.
- Stop conditions: Stop once the normal scenario INI stream priority for an already selected ordinary map token is verified; record MIX linked-list order and runtime debugger/file experiments as uncertainty if needed.

## 1. Overview

The selected scenario record does not carry a pinned source handle from the Choose Map source builder into game load. The selected loader opens record `+0x58` only as an availability check, then copies the original token string into `DAT_00A8B8E0` and `ScenarioClass+0x125C`.

For ordinary non-random maps, the real scenario parse later constructs a new `CCFileClass` for that filename and streams it through `SHAPipe`. That actual read path checks whether a raw filesystem file exists first; if it does, it opens that raw file and skips MIX lookup. If no raw file exists, it looks up the normalized filename in the loaded MIX list. Therefore a loose file with the same selected `+0x58` token shadows the archive-contained map payload at scenario parse time.

## 2. Key Offsets / Functions

| Item | Meaning in this slice | Evidence | Active in YR |
|---|---|---|---|
| record `+0x58` | ASCII selected map token/path, e.g. PKT-derived `.MAP` name or direct loose `.YRM` name | `0x005E7C2B..0x005E7C3B`; prior source/selected reports | Yes |
| `DAT_00A8B8E0` | selected map token global copied from record `+0x58` | `0x005E7D87..0x005E7D9E`; prior selected report | Yes |
| `ScenarioClass+0x125C` | scenario filename buffer passed into post-shell load | `0x005E7DA0..0x005E7DCA`; `0x0052E737..0x0052E745`; `0x00683AB0` | Yes |
| `CCFileClass__Constructor @ 0x004739F0` | initializes file object and calls filename setup `0x0047AE10` | decompile `0x004739F0`; call sites `0x00686797..0x0068679F` | Yes |
| `FUN_0047AE10` | set-name/search-path setup; probes raw filesystem via `FUN_00431F10` before trying path prefixes | decompile `0x0047AE10` | Yes |
| `FUN_00473C50` | availability check used by selected loader; can succeed from MIX or raw | assembly `0x00473C50..0x00473CB5`; decompile `0x00473C50` | Yes |
| `FUN_00473D10` | actual open/read setup; raw filesystem hit is preferred before MIX fallback | decompile `0x00473D10`; assembly `0x00473D1D..0x00473E40` | Yes |
| `FUN_005B4430` | MIX member lookup by normalized filename CRC over loaded MIX linked list `DAT_00ABEFE0` | decompile `0x005B4430` | Yes |
| `FUN_00431F10` / `FUN_0065CBF0` | raw filesystem availability probe using `CreateFileA(..., OPEN_EXISTING, ...)` | decompile `0x00431F10`, `0x0065CBF0` | Yes |
| `FUN_0065CB50` | raw filesystem open using `CreateFileA` in read/write modes | decompile `0x0065CB50` | Yes |

## 3. Core Logic

### 3.1 Selected-record loader checks availability but copies only the token

`0x005E7BF0(index)` constructs a `CCFileClass` from `DAT_00A8B8CC[index] + 0x58`, then calls vtable `+0x14` with `0`, which resolves to `FUN_00473C50` for the `CCFileClass` vtable at `0x007E16B0`. If the availability call fails, the selected loader returns failure before copying the record data. If it succeeds, the loader copies record metadata, then copies record `+0x58` into `DAT_00A8B8E0` and from there into `ScenarioClass+0x125C`.

Active in YR: Yes. Evidence: `0x005E7C2B..0x005E7C49` constructs the `CCFileClass` and calls vtable `+0x14`; prior selected-map report verifies `0x005E7D87..0x005E7DCA` token copies.

The loader does not preserve the resolved file source. It destroys the temporary file object after the availability check and stores only the original filename/token string. This is why source priority must be determined again at the later scenario parse open.

Active in YR: Yes. Evidence: `0x005E7C4B..0x005E7C73` destructor/base cleanup after the check; later copies come from record string/global string, not from a file handle.

### 3.2 Availability check can be satisfied by MIX before raw, but it is not the bytes source

`FUN_00473C50` first returns true if the file object already has open/buffer status. On a fresh selected-loader object, it calls vtable `+0x4` to get the filename, calls `FUN_005B4430`, and only if that MIX lookup fails calls `FUN_00431F10(0)` to test raw filesystem availability. This means the selected loader can accept an archive-backed stock map even when no loose file exists.

Active in YR: Yes. Evidence: assembly `0x00473C65..0x00473C90` checks current state and calls `FUN_005B4430`; `0x00473C9E..0x00473CB5` probes raw and writes available/unavailable state.

This order is load-gating only. No selected source pointer or MIX entry is copied into `ScenarioClass+0x125C`. The later scenario parse opens a new object.

Active in YR: Yes. Evidence: selected loader behavior above plus scenario parse call chain below.

### 3.3 Post-shell ordinary scenario parse constructs a fresh file object

For standard non-campaign Skirmish, `Main_Game` passes `ScenarioClass+0x125C` to `ScenarioClass__Start_Scenario`. `ScenarioClass__Start_Scenario` constructs a `CCFileClass` and a `SHAPipe` for intro/briefing metadata, then later calls `ScenarioClass__Read_Scenario`. For ordinary non-`.SED` maps, `ScenarioClass__Read_Scenario` calls `ScenarioClass__Read_Scenario_INI @ 0x00686730`, which constructs another `CCFileClass` from the same filename and streams it through `SHAPipe`.

Active in YR: Yes for ordinary selected maps; Conditional for the `.SED` random branch, which is routed elsewhere. Evidence: prior selected report for `0x0052E737..0x0052E745`; `0x00683C4E..0x00683C67` constructor/`SHAPipe` call; `0x006849C9` calls `0x00686730`; `0x00686797..0x006867BE` constructs `CCFileClass` then `SHAPipe`.

### 3.4 Actual read/open priority is raw filesystem first, then MIX fallback

The `CCFileClass` vtable used in the scenario path has `FUN_00473D10` at vtable `+0x1C`, the open routine used by reads. In read mode (`param_2 & 2 == 0`), `FUN_00473D10` first probes the raw filesystem with `FUN_00431F10(0)`. If that returns true, the function jumps to `FUN_0047AAB0(param_2)`, which opens the raw file through `FUN_0065CB50`.

Active in YR: Yes. Evidence: decompile `0x00473D10`; assembly `0x00473D1D..0x00473D34` tests write bit and probes raw; `0x00473E39..0x00473E40` calls `FUN_0047AAB0`.

Only when the raw probe returns false does `FUN_00473D10` call `FUN_005B4430` to search loaded MIX archives. On MIX hit, it initializes an in-memory buffer for the archive member and returns success. On MIX miss, it falls back to `FUN_0047AAB0(param_2)`, which attempts raw open and fails normally if no raw file exists.

Active in YR: Yes. Evidence: decompile `0x00473D10`; assembly `0x00473D3A..0x00473DBD` covers the MIX lookup and buffer setup before the fallback open.

Therefore, when both `record+0x58 = "X.MAP"` exists in a loaded MIX archive and a loose `X.MAP` exists in the filesystem/search path, the actual scenario INI bytes come from the loose file.

Active in YR: Conditional on duplicate loose/archive token. Evidence: `0x00473D10` raw-before-MIX branch order plus `0x00686730` scenario INI file stream call chain.

### 3.5 Constructor raw path/search behavior

`CCFileClass__Constructor @ 0x004739F0` calls `FUN_0047AE10(filename)`. That setup first stores/tests the provided name through the lower file layer. If the raw filesystem probe succeeds, setup returns immediately. If raw does not exist and `DAT_0089E410` search-path entries exist, it tries prefix + filename paths until a raw file exists; otherwise it reverts to the original token.

Active in YR: Yes. Evidence: decompile `0x004739F0`; `FUN_0047AE10` calls `FUN_00431E80(param_2)`, probes `FUN_00431F10(0)`, iterates `DAT_0089E410`, tries prefixed names, and falls back to `FUN_00431E80(param_2)`.

This reinforces the loose-shadow rule: a loose file in the current or configured search path becomes the file object's raw filename before the later read/open routine runs.

Active in YR: Conditional on configured search paths and duplicate filename. Evidence: `FUN_0047AE10`; raw probe/open functions `0x00431F10`, `0x0065CBF0`, `0x0065CB50`.

## 4. INI / Asset Inputs

No rules/art INI key controls this priority. The inputs are selected scenario-record tokens from PKT/YRM source population and the file system/MIX state at load time.

| Input | Effect | Evidence | Active in YR |
|---|---|---|---|
| `MISSIONSMD.PKT [MultiMaps]` entry value | PKT record constructor appends `.MAP` into record `+0x58`; later selected open resolves this token | prior source report `0x0069A3B0` | Yes |
| loose `.YRM` record filename | direct record constructor stores the loose path/name in record `+0x58` | prior source report `0x0069A980` | Yes |
| loaded MIX list `DAT_00ABEFE0` | searched by `FUN_005B4430` after filename normalization and CRC | decompile `0x005B4430` | Yes |
| raw filesystem/search path | probed/opened via `CreateFileA` in `FUN_0065CBF0` / `FUN_0065CB50`; preferred by actual read/open | decompile `0x0065CBF0`, `0x0065CB50`, `0x00473D10` | Conditional on file presence |

## 5. Integration Points

| Integration | Behavior | Evidence | Active in YR |
|---|---|---|---|
| Choose Map source list | provides record pointer and `+0x58` token; duplicate rows are not deduped | prior `SKIRMISH_SCENARIO_SOURCE_POPULATION_ORDER_GHIDRA_REPORT.md` | Yes |
| Choose Map accept/selected loader | calls `0x005E7BF0(index)`; failure restores prior selection in parent path | prior selected/accept reports; `0x005E7C2B..0x005E7C49` | Yes |
| Selected loader availability check | accepts either MIX or raw availability but stores only token | `0x00473C50`, `0x005E7D87..0x005E7DCA` | Yes |
| Scenario start | receives `ScenarioClass+0x125C`, constructs fresh `CCFileClass` | `0x00683AB0`; `0x00683C4E..0x00683C67` | Yes |
| Scenario INI read | constructs fresh `CCFileClass`, streams via `SHAPipe`, then full-inits scenario | `0x00686730`; `0x00686797..0x006867BE` | Yes for non-`.SED` |
| File-open priority | raw filesystem/read path first; MIX fallback if raw absent | `0x00473D10`, `0x005B4430`, `0x0065CB50` | Yes |

## 6. Current Rust Implementation Status

Rust currently does not model the selected-record token re-resolution against an asset system with native loose-before-MIX priority.

| Area | Current Rust status | Evidence |
|---|---|---|
| map discovery | `src/app_list_maps.rs` scans loose filesystem entries via `read_dir`; prior reports note this differs from retail source population | `rg` output for `std::fs::read_dir` |
| concrete records | `SkirmishScenarioRecord` carries `file_name` and source ordinal, which can support token identity | `src/skirmish_scenarios.rs` |
| launch session | `SkirmishLaunchSession.selected_map_file` stores the selected file string | `src/skirmish_launch.rs`, `src/ui/skirmish_shell/state.rs` |
| map load | `load_map_by_name_or_path` loads by path/name from the RA2 directory; archive-backed PKT `.MAP` payload lookup and loose-before-MIX collision tests are not modeled | `src/app_list_maps.rs`, `src/app_init.rs` |
| asset archives | `app_init.rs` loads MIX assets for other systems, but the current map load path does not expose the native `CCFileClass` selected-map lookup contract | `src/app_init.rs` |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| prior source-population duplicate-token boundary | verified-from-prior | `SKIRMISH_SCENARIO_SOURCE_POPULATION_ORDER_GHIDRA_REPORT.md` OQ-11 | no re-investigation done |
| selected loader construction/check | verified | `0x005E7C2B..0x005E7C49`; `0x00473C50` | none |
| selected loader token copy/no source pin | verified | `0x005E7D87..0x005E7DCA`; destructor cleanup around `0x005E7C4B..0x005E7C73` | none |
| post-shell scenario start open | verified | `0x00683AB0`; `0x00683C4E..0x00683C67` | none for standard offline |
| ordinary scenario INI open | verified | `0x00686730`; `0x00686797..0x006867BE` | `.SED` random branch out of scope |
| `CCFileClass` constructor/set-name | verified | `0x004739F0`, `0x0047AE10` | exact runtime search-path contents not enumerated |
| availability check priority | verified | `0x00473C50..0x00473CB5`; `0x005B4430`; `0x00431F10` | none for gating behavior |
| actual read/open priority | verified | `0x00473D10`; `0x0065CB50`; `0x005B4430` | none for loose-vs-MIX priority |
| MIX linked-list archive order | touched-not-exhausted | `0x005B4430` walks `DAT_00ABEFE0` | exact load order belongs to archive-system investigation |
| runtime duplicate loose/archive experiment | deferred | static proof sufficient for branch order | optional debugger/filesystem confirmation |
| current Rust delta | verified-scan | `rg`/Codegraph over listed Rust files | implementation not performed |

## 8. Open Questions - Final State of Investigation Log

- `[RESOLVED] OQ-01 - Does selected record +0x58 reach an availability open? -> Yes, 0x005E7BF0 constructs CCFileClass from record+0x58 and calls vtable +0x14.` (evidence: `0x005E7C2B..0x005E7C49`)
- `[RESOLVED] OQ-02 - Does selected loader store a resolved source handle? -> No, it stores/copies the original token string into DAT_00A8B8E0 and ScenarioClass+0x125C.` (evidence: `0x005E7D87..0x005E7DCA`)
- `[RESOLVED] OQ-03 - What does selected-loader vtable +0x14 do? -> It is the CCFile availability check; it can accept already-open, MIX, or raw availability.` (evidence: `0x00473C50..0x00473CB5`; vtable assignment `0x005E7C4F = 0x007E16B0`)
- `[RESOLVED] OQ-04 - Is the availability check the final source choice? -> No, the temp object is cleaned up and scenario parse later constructs a fresh CCFileClass.` (evidence: `0x005E7C4B..0x005E7C73`; `0x00686797..0x006867BE`)
- `[RESOLVED] OQ-05 - Which path parses ordinary map INI bytes? -> ScenarioClass__Read_Scenario_INI constructs CCFileClass and SHAPipe, then calls Full_Init on success.` (evidence: `0x00686730`, `0x00686797..0x006867BE`)
- `[RESOLVED] OQ-06 - Does actual read/open prefer loose filesystem or MIX? -> Loose/raw filesystem; FUN_00473D10 checks FUN_00431F10 first and jumps to raw open on success before MIX lookup.` (evidence: `0x00473D10`, `0x00473D1D..0x00473E40`)
- `[RESOLVED] OQ-07 - What happens if no loose/raw file exists? -> FUN_00473D10 calls FUN_005B4430 to search loaded MIX entries and buffers the archive member on hit.` (evidence: `0x00473D10`; `0x005B4430`)
- `[RESOLVED] OQ-08 - How is raw filesystem existence/open implemented? -> CreateFileA probes/open calls in FUN_0065CBF0 and FUN_0065CB50.` (evidence: `0x0065CBF0`, `0x0065CB50`)
- `[RESOLVED] OQ-09 - Does CCFile setup try path prefixes? -> Yes, FUN_0047AE10 tests raw existence, then tries DAT_0089E410 prefix entries, then falls back to original token.` (evidence: `0x0047AE10`)
- `[RESOLVED] OQ-10 - Is `.SED` random generation part of this claim? -> No, ordinary non-SED calls ScenarioClass__Read_Scenario_INI; .SED branch is covered by the random-map report.` (evidence: `0x006849C9`; `SKIRMISH_RANDOM_MAP_BRANCH_AFTER_SELECTED_MAP_LOAD_GHIDRA_REPORT.md`)
- `[RESOLVED] OQ-11 - Does a loose `.MAP` shadow an archive `.MAP` with same selected token? -> Yes for actual scenario parse, because the selected source is not pinned and the later CCFile read/open chooses raw before MIX.` (evidence: `0x005E7D87..0x005E7DCA`, `0x00686797..0x006867BE`, `0x00473D10`)
- `[RESOLVED] OQ-12 - Does Rust currently model this priority? -> No; map load is path/name based and does not expose archive-backed selected-map payload lookup with loose-before-MIX collision behavior.` (evidence: `src/app_list_maps.rs`, `src/app_init.rs`, Codegraph scan)
- `[DEFERRED] OQ-13 - Exact order of MIX archives when multiple archives contain the same filename.` (category: out-of-scope; reason: this slice proves loose-vs-MIX priority; archive list population order is a separate asset-system investigation; next-step-if-pursued: trace MIX registration into `DAT_00ABEFE0`)
- `[DEFERRED] OQ-14 - Runtime screenshot/debugger confirmation with a synthetic duplicate loose/archive map.` (category: needs-runtime-debugger; reason: static branch order is sufficient for implementation handoff, but runtime experiment would verify local install/search-path state; next-step-if-pursued: create a duplicate `MULTI*.MAP`, breakpoint `0x00473D10`, and observe raw branch)
- `[DEFERRED] OQ-15 - Exact contents/order of `DAT_0089E410` search paths during standard Skirmish.` (category: out-of-scope; reason: constructor prefix probing is verified, but path-list population is not required to answer loose-vs-MIX precedence; next-step-if-pursued: trace search-path list setup before shell entry)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Selected record `+0x58` is re-resolved at scenario parse; selected loader does not pin archive/source. | `0x005E7BF0`; `0x005E7D87..0x005E7DCA`; `0x00686730` | missing source-resolution contract | `src/skirmish_scenarios.rs`, `src/skirmish_launch.rs`, `src/app_init.rs`, `src/app_list_maps.rs` | Keep selected record identity/token, but resolve bytes at load through the native priority resolver rather than through chooser source identity. | Stock PKT record selected, then same-name loose map exists at load; game parses loose file bytes. Proposed test: `skirmish_selected_map_token_re_resolves_at_load_time` | Do not pin a MIX member handle from source population/chooser into launch. |
| Actual ordinary scenario read priority is loose/raw filesystem first, then MIX fallback. | `0x00473D10`; `0x0065CB50`; `0x005B4430` | missing archive-backed fallback plus collision priority | map-loading resolver in `src/app_list_maps.rs` / `src/app_init.rs`; asset manager map payload API | Implement selected-map load as raw `record+0x58`/search path first, then loaded MIX lookup by normalized token. | Fixture with `MULTI01.MAP` in fake MIX and loose `MULTI01.MAP` loads the loose content. Proposed test: `skirmish_loose_map_shadows_archive_map_with_same_token` | Do not prefer archive records over loose files just because the chooser row came from `MISSIONSMD.PKT`. |
| Availability check can succeed from MIX even without loose file, and scenario parse later uses MIX when loose is absent. | `0x00473C50`; `0x00473D10`; `0x005B4430` | missing stock archive map load | `src/app_list_maps.rs`, `src/app_init.rs`, future MIX-backed map loader | Archive-only stock `record+0x58` tokens must load from MIX when no loose file shadows them. | Fixture with only archive `MULTI01.MAP` loads successfully from archive. Proposed test: `skirmish_archive_map_loads_when_no_loose_shadow_exists` | Do not require every PKT map token to exist as a loose root file. |
| `CCFileClass` setup can prefer a raw file found through configured search paths before reverting to original token/MIX fallback. | `0x0047AE10`; `0x00431F10`; `0x0065CB50` | unchecked path-prefix model | asset/file resolver layer | Search-path loose files should participate in the same raw-before-MIX priority when those paths are modeled. | With a configured user-map search path containing `MULTI01.MAP`, selected stock token loads that search-path file. Proposed test: `skirmish_search_path_loose_map_shadows_archive_token` | Do not hardcode only the install root if the resolver later models native search paths. |

### Negative Facts / Do Not Do

- Do not treat source-population order as source-locking. The source builder may create an archive-backed row, but later load stores and re-resolves only `record+0x58`. Evidence: `0x005E7D87..0x005E7DCA`, `0x00686797..0x006867BE`.
- Do not prefer MIX/archive bytes over a loose same-name `.MAP` during ordinary scenario parse. Evidence: `FUN_00473D10` raw probe/open path precedes `FUN_005B4430`.
- Do not use the selected-loader availability check order as the final parse-source order. Evidence: `FUN_00473C50` checks MIX before raw for availability, while `FUN_00473D10` uses raw before MIX for actual open.
- Do not require stock `MISSIONSMD.PKT` maps to exist as loose files. Evidence: `FUN_00473C50` and `FUN_00473D10` both use `FUN_005B4430` as archive fallback/availability.
- Do not claim exact duplicate resolution between two MIX archives from this report. Evidence: `FUN_005B4430` walks `DAT_00ABEFE0`, but this report did not trace archive registration order.

## Remaining Uncertainty

- Exact order among multiple loaded MIX archives containing the same filename remains out of scope; it requires tracing `DAT_00ABEFE0` population.
- Exact runtime contents of `DAT_0089E410` search paths during standard Skirmish were not enumerated; constructor prefix probing is verified.
- A runtime duplicate-map experiment would be useful for local install validation, but static evidence already proves branch priority.

## Stale Docs / Follow-up Docs

- `docs/research/skirmish-ui/SKIRMISH_SCENARIO_SOURCE_POPULATION_ORDER_GHIDRA_REPORT.md`: replace the OQ-11 deferral with:

  > [RESOLVED by `SKIRMISH_SELECTED_MAP_FILE_OPEN_PRIORITY_LOOSE_SHADOWING_GHIDRA_REPORT.md`] Later selected-file loading does let a loose filesystem map shadow an archive map with the same selected record `+0x58` token for ordinary non-`.SED` Skirmish maps. `0x005E7BF0` uses a temporary `CCFileClass` availability check but stores only the original token into `DAT_00A8B8E0` / `ScenarioClass+0x125C`. The later scenario INI path constructs a fresh `CCFileClass`; its actual read/open routine `0x00473D10` probes/opens raw filesystem files before falling back to `FUN_005B4430` MIX lookup. Archive-vs-archive duplicate order remains a separate MIX registration question.

- `docs/research/skirmish-ui/SKIRMISH_SELECTED_MAP_TOKEN_LOAD_CONSUMER_GHIDRA_REPORT.md`: add after the first-file-open discussion:

  > The selected-loader open is only an availability gate and does not pin the source. For ordinary non-`.SED` maps, the scenario INI bytes are resolved later by a fresh `CCFileClass`/`SHAPipe` path; raw loose files with the same token are preferred before MIX fallback.

## Sources

- Ghidra read-only decompile / assembly: `0x005E7BF0`, `0x00683AB0`, `0x00684620`, `0x00686730`, `0x004739F0`, `0x0047AE10`, `0x00473C50`, `0x00473D10`, `0x005B4430`, `0x00431F10`, `0x0065CBF0`, `0x0065CB50`, `0x004741F0`.
- Ghidra assembly context: `0x005E7C2B..0x005E7C73`, `0x00683C4E..0x00683C67`, `0x00686797..0x006867BE`, `0x00473C50..0x00473CB5`, `0x00473D1D..0x00473E40`.
- Prior docs: `docs/research/skirmish-ui/SKIRMISH_SCENARIO_SOURCE_POPULATION_ORDER_GHIDRA_REPORT.md`; `docs/research/skirmish-ui/SKIRMISH_SELECTED_MAP_TOKEN_LOAD_CONSUMER_GHIDRA_REPORT.md`; `docs/research/skirmish-ui/SKIRMISH_RANDOM_MAP_BRANCH_AFTER_SELECTED_MAP_LOAD_GHIDRA_REPORT.md`.
- Rust scan: `src/app_list_maps.rs`; `src/app_init.rs`; `src/skirmish_scenarios.rs`; `src/skirmish_launch.rs`; `src/ui/skirmish_shell/state.rs`.
