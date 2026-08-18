# Skirmish Random Map Branch After Selected Map Load - Ghidra Research Report

**Address(es):** `0x00684620`, `0x00597A10`, `0x00598960`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** The `.SED` random-map branch after `ScenarioClass__Read_Scenario @ 0x00684620` receives the shell-selected `RandMap.Sed` filename from `ScenarioClass+0x125C`, invokes the random-map seed/load wrapper, conditionally runs generation, and writes the scenario filename/state used by the rest of the load.  
**Non-Scope:** Random terrain formulas, random-map dialog visuals, Choose Map preview refresh, real listbox/combo owner-draw behavior, trackbar disabled flow, start-marker clipping, online/WOL variants, and full random-map generator internals beyond immediate launch contract.  
**Confidence:** High for branch predicate, call order, filename retention, and current Rust delta; Medium for the precise vtable target behind the `0x00597A10` seed-load dispatch because the direct call is verified but the vtable table itself was not dumped in this read-only pass.  
**Active in YR:** Conditional. Active in standard YR offline Skirmish when the selected scenario record has filename suffix `.SED` such as `RandMap.Sed`; inactive for ordinary stock/custom map filenames. Evidence: shell reports verify `0x005E7BF0` copies selected record `+0x58` into `ScenarioClass+0x125C`; `0x00684694..0x006846BE` sets `ScenarioClass+0x34BD` by comparing that filename suffix with `.SED`; `0x00684961..0x00684990` takes the random branch when `+0x34BD != 0`.

## Working Notes

- Target question: after the shell-selected random sentinel reaches scenario load, does native YR replace it with a generated filename/state, and what should Rust launch do?
- Non-goals: random terrain formulas, modal preview behavior, listbox/combo paint, trackbar enable flow, and marker clipping.
- Evidence needed to mark COMPLETE: decompile plus assembly for `0x00684620` random branch, decompile plus caller evidence for `0x00597A10`, immediate `0x00598960` call evidence, prior-doc handoff evidence from Choose Map/Create Random Map, and Rust surface scan.
- Stop conditions: stop at immediate generator handoff once filename/state/order are verified; list generator internals as open questions unless directly required for launch contract.

## 1. Overview

The shell handoff for a random map is not a special negative index and does not switch to a generated loose `.map` filename. The accepted Choose Map record still contributes a normal filename token, `RandMap.Sed`, which reaches `ScenarioClass__Read_Scenario` through `ScenarioClass+0x125C`.

`ScenarioClass__Read_Scenario` detects randomness by comparing the filename suffix to `.SED`, sets `ScenarioClass+0x34BD`, calls `0x00597A10` to load/apply the seed from the `.SED` filename, then calls `0x00598960(0,0)` only if the seed step succeeds. After the random branch, it copies the original local filename buffer back into `ScenarioClass+0x125C`; the retained scenario filename is still `RandMap.Sed`.

## 2. Class Layout / Key Offsets

| Field / global | Meaning in this slice | Evidence | Active in YR |
|---|---|---|---|
| `ScenarioClass+0x125C` | Scenario filename buffer; contains `RandMap.Sed` after selected-record loader and remains `RandMap.Sed` after random generation branch copy-back | `0x005E7BF0` prior report; `0x00684995..0x006849BF` | Conditional: Yes for random sentinel, Yes for normal filename retention |
| `ScenarioClass+0x34BD` | `IsRandom` byte set by `.SED` suffix detection | `0x00684694..0x006846BE`; read at `0x00684961` | Conditional: true only for `.SED` suffix |
| `ScenarioClass+0x3598` | load-in-progress flag set at scenario-read entry and cleared before return/error; also changes progress behavior in generator | write `0x00684675`, final clear in `0x00684620`; reads in `0x00598960` | Yes |
| `0x00ABDFD8` | random-map seed/generator object passed as `ECX` to `0x00597A10` and `0x00598960` | `0x0068496F`, `0x00684984`; constructor-related copies in `0x00596300` | Conditional: random branch uses it |
| `MapSeed +0x74` | seed value consumed by `0x00598960` through `FUN_0065C6D0(*(this+0x74))` | `0x0059897B`; constructor/randomizer writes in `0x00595680`, `0x00597260`, `0x00597380`, `0x00597430` | Conditional |
| `MapSeed +0x38` | theater/desert-snow/temperate class input used by generator and later lighting/theme setup | `0x00598960`; `0x00599650` | Conditional |
| `MapSeed +0x3C` | map size/type bucket used by generator branch choices; values `3`/`4` take the water/region variant | `0x00598960` branch checks | Conditional |
| `ScenarioClass+0x1258` | theater id updated by generator initialization when generated theater differs | `0x00599650` | Conditional |
| `ScenarioClass+0x3528/+0x3534/+0x3538/+0x353C/+0x3544` | lighting outputs written from generated map settings near the end of `0x00599650` | `0x00599650` | Conditional |

## 3. Core Logic

### Filename predicate and random flag

`ScenarioClass__Read_Scenario` first copies its input filename into a local 260-byte buffer. It then measures the copied string, computes a pointer near the final four characters, and compares that suffix with string `.SED` at `0x0083DA88` via `0x007C8D20`. Active in YR: Yes / Conditional. Evidence: decompile `0x00684620`; assembly `0x0068465C..0x00684694`.

If the suffix compare returns equal, it writes `ScenarioClass+0x34BD = 1` and logs the `Scen->IsRandom = true` string. Otherwise it writes `ScenarioClass+0x34BD = 0` and the normal map path later calls `ScenarioClass__Read_Scenario_INI @ 0x00686730`. Active in YR: Yes / Conditional. Evidence: assembly `0x0068469C..0x006846BE`, normal branch call at `0x006849C9`.

The suffix compare is generic `.SED`, not a full-case special compare against `RandMap.Sed`. In standard YR Skirmish the only verified shell-created sentinel filename is `RandMap.Sed`, so the branch is active through that shell-selected record. Active in YR: Conditional. Evidence: `.SED` compare at `0x0068465C/0x00684694`; sentinel creation and selected-record handoff in prior `SKIRMISH_RANDMAP_SED_RANDOM_MAP_BEHAVIOR_GHIDRA_REPORT.md` and `SKIRMISH_SELECTED_MAP_TOKEN_LOAD_CONSUMER_GHIDRA_REPORT.md`.

### Branch order after `.SED`

When `ScenarioClass+0x34BD != 0`, `0x00684620` loads `ECX = 0x00ABDFD8`, takes `EDX = &local_filename`, pushes that filename pointer, and calls `0x00597A10`. Active in YR: Conditional on `.SED`. Evidence: assembly `0x00684961..0x00684975`.

`0x00597A10` is a tiny wrapper: if the stack argument is non-null, it dispatches `this->vtable[1]` / vtable `+0x4` with the filename pointer and returns that byte result; if the argument is null, it calls `0x005587F0` and returns whether that succeeded. The random-branch caller always passes a non-null local filename, so standard `RandMap.Sed` launch uses the vtable load path, not the null/default path. Active in YR: Conditional. Evidence: decompile `0x00597A10`; assembly `0x00597A10..0x00597A2B`; caller assembly `0x0068496B..0x00684975`.

If `0x00597A10` returns false, `0x00684620` skips `0x00598960` and continues to the common filename copy-back and later error path with the failed return byte. Active in YR: Conditional failure path. Evidence: assembly `0x0068497A..0x00684995`; error handling later in `0x00684620`.

If `0x00597A10` returns true, `0x00684620` calls `0x00598960` with `ECX = 0x00ABDFD8` and two zero stack arguments, then calls `ScenarioClass__Post_Map_Init @ 0x00686890` with `CL = 1`. Active in YR: Conditional success path. Evidence: assembly `0x0068497C..0x00684990`; decompile `0x00684620`.

### Filename/state after generation

After either success or failure of the random generator wrapper, `0x00684620` copies the same local filename buffer back into `ScenarioClass+0x125C`. There is no observed overwrite to `RandMap.Map`, a generated `.map`, or a display-name-derived filename in this branch. Active in YR: Conditional. Evidence: assembly `0x00684995..0x006849BF`; decompile `0x00684620`.

The generator work is in-memory. `0x00598960(0,0)` initializes generated-map state, builds terrain/cells/regions/starts/resources, recalculates cell attributes, initializes tiberium queues, radar bounds, and lighting/theater state. It does not need to return a generated filename to the scenario loader. Active in YR: Conditional. Evidence: `0x00598960` decompile and immediate callees; filename copy-back remains `local_filename` at `0x00684995..0x006849BF`.

`0x00598960` treats `param_2 == 0` differently from preview/dialog generation: the repeated `GenerateTerrainPreview(); SendMessageA(hwnd, WM_PAINT, 0, 0)` blocks are guarded by `(char)param_2 != 0`, and the scenario-load call passes `0`, so launch generation does not update the Choose Map preview window while loading. Active in YR: Yes / Conditional. Evidence: caller pushes `0,0` at `0x00684980..0x00684989`; guarded preview blocks in `0x00598960`; dialog path calls `0x00598960(1, hwnd)` in `0x00596300`.

### Active standard YR path

The path is live in standard offline Skirmish because previous reports verify Create Random Map command `0x583 -> 0x005E8590` creates/updates a normal scenario record whose `+0x58` filename is `RandMap.Sed`, Choose Map accept commits the ordinary record index, and `0x005E7BF0(index)` copies that filename into `ScenarioClass+0x125C`. `ScenarioClass__Read_Scenario` then performs the `.SED` suffix test on that same string. Active in YR: Yes / Conditional by user selecting/accepting the random sentinel. Evidence: prior reports plus `0x00684620` / `0x00684961..0x00684990`.

## 4. INI Keys

| File / section / key | Value / meaning | Evidence | Active in YR |
|---|---|---|---|
| `ini/mpmodesmd.ini:[Battle] 1` fifth field | `true`; Battle allows the random sentinel into Choose Map after creation | `ini/mpmodesmd.ini`; prior mode parser report; `0x005D6350` in prior random-map report | Yes |
| `ini/mpmodesmd.ini:[FreeForAll] 2` fifth field | `true`; Free For All allows random sentinel | same | Conditional by selected mode |
| `ini/mpmodesmd.ini` ids `3,4,5,6,7,8,9` fifth field | `false`; random sentinel filtered out in those modes | same | Conditional by selected mode |
| `ini/rulesmd.ini:[MultiplayerDialogSettings]` | launch options defaults, not the `.SED` branch selector | local INI scan; no reader in `0x00684620` branch | Yes as setup defaults, not this branch |
| RMG settings strings such as `RMGLevelLightSettings` | consumed inside `0x005981F0` / `0x00599650` generator support, not part of shell-selected filename handoff | decompile `0x005981F0`, `0x00599650`; `rg RMG` found no stock `rulesmd.ini` keys | Conditional generator internals; formulas out of scope |

## 5. Integration Points

| Integration | Behavior | Evidence | Active in YR |
|---|---|---|---|
| Choose Map/Create Random Map output | creates/updates normal scenario record with file token `RandMap.Sed`; accept commits ordinary record index | prior `SKIRMISH_RANDMAP_SED_RANDOM_MAP_BEHAVIOR_GHIDRA_REPORT.md` | Conditional on random command and allowed mode |
| Selected-record loader | copies record `+0x58` into `ScenarioClass+0x125C` before Start/load | prior `SKIRMISH_SELECTED_MAP_TOKEN_LOAD_CONSUMER_GHIDRA_REPORT.md` | Yes / Conditional for sentinel |
| `ScenarioClass__Read_Scenario` predicate | `.SED` suffix sets `IsRandom` byte | `0x00684694..0x006846BE` | Conditional |
| Seed-load wrapper | `0x00597A10` receives object `0x00ABDFD8` plus filename pointer; non-null path dispatches vtable `+0x4` | `0x0068496B..0x00684975`, `0x00597A10..0x00597A2B` | Conditional |
| Generator invocation | success calls `0x00598960(0,0)` and `ScenarioClass__Post_Map_Init(1)` | `0x0068497C..0x00684990` | Conditional |
| Common filename write-back | copies original local filename into `ScenarioClass+0x125C` after random branch | `0x00684995..0x006849BF` | Conditional |
| Normal map branch | calls `ScenarioClass__Read_Scenario_INI`, which opens the filename and also copies it to `ScenarioClass+0x125C` | `0x006849C3..0x006849C9`, `0x00686730` | Yes for non-SED |

## 6. Current Rust Implementation Status

| Area | Current Rust status | Evidence |
|---|---|---|
| scenario records | `SkirmishScenarioRecord` already has `SkirmishScenarioKind::RandomMapSentinel` and `RANDMAP_SED = "RandMap.Sed"` | `C:/Users/enok/Documents/ra2-rust-game/src/skirmish_scenarios.rs` |
| random sentinel filtering/upsert | mode-gated sentinel insertion/update exists; current random sentinel has no min/max `2/4` parity in the struct constructor | `src/skirmish_scenarios.rs`, `src/ui/skirmish_shell/state.rs` |
| launch session | `SkirmishLaunchSession.selected_map_file` stores a string and Start passes it into `GameScreen::Loading` | `src/skirmish_launch.rs`, `src/ui/skirmish_shell/state.rs`, `src/app.rs` |
| map loading | `load_map_by_name_or_path` searches real files/extensions and will fail or attempt normal file loading for `RandMap.Sed`; no `.sed`/random-generation branch exists | `src/app_list_maps.rs`, `src/app_init.rs` |
| app start legacy path | main-menu Start still reads `available_maps[selected_map_idx].file_name`; dev shell session path uses `selected_map_file` | `src/app.rs` |

Rust has part of the shell record model, but the post-load branch is missing: selecting the random sentinel should not try to parse `RandMap.Sed` as a normal map file.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `ScenarioClass__Read_Scenario @ 0x00684620` `.SED` predicate | verified | decompile and assembly `0x0068465C..0x006846BE` | none |
| random branch caller setup | verified | assembly `0x00684961..0x00684990` | none |
| `0x00597A10` seed-load wrapper | verified | decompile and assembly `0x00597A10..0x00597A2B`; caller evidence | exact vtable table dump deferred, but dispatch shape is verified |
| `0x00598960(0,0)` launch invocation | verified | caller assembly `0x00684980..0x00684989`; decompile `0x00598960` | terrain formulas out of scope |
| random branch filename copy-back | verified | assembly `0x00684995..0x006849BF` | none |
| `ScenarioClass__Post_Map_Init(1)` after successful generation | verified | assembly `0x0068498E..0x00684990`; decompile `0x00686890` | deeper spawn internals covered by other reports |
| preview/dialog generation distinction | verified | caller passes `0,0`; `0x00598960` preview blocks require `param_2 != 0`; dialog `0x00596300` passes `1, hwnd` | exact dialog visual UX out of scope |
| RMG terrain formulas | deferred | `0x00598960` callees | out-of-scope dedicated random-map-generator report |
| Rust launch delta | verified | codegraph + `rg` + file reads | implementation not performed in this report |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] OQ-RB-01 - Which filename reaches this branch? -> The selected record filename already copied into `ScenarioClass+0x125C`, `RandMap.Sed` for the random sentinel.` (evidence: prior selected-map report; `0x00684620`)
- `[RESOLVED] OQ-RB-02 - What is the random predicate? -> Suffix compare with `.SED`, not a full string compare to `RandMap.Sed`.` (evidence: `0x0068465C..0x00684694`)
- `[RESOLVED] OQ-RB-03 - What field records random state? -> `ScenarioClass+0x34BD` is set to `1` on suffix match and `0` otherwise.` (evidence: `0x0068469C..0x006846BE`)
- `[RESOLVED] OQ-RB-04 - Is the branch active in standard YR? -> Yes, conditionally when the shell-created random sentinel is accepted in a random-map-allowed mode.` (evidence: prior random-map report; `ini/mpmodesmd.ini`; `0x00684961`)
- `[RESOLVED] OQ-RB-05 - What does `0x00597A10` receive? -> `ECX=0x00ABDFD8`, stack arg `&local_filename`; the caller passes a non-null filename.` (evidence: `0x0068496B..0x00684975`)
- `[RESOLVED] OQ-RB-06 - What does `0x00597A10` do on this path? -> non-null argument dispatches object vtable `+0x4` with the filename and returns that result.` (evidence: `0x00597A10..0x00597A1E`)
- `[RESOLVED] OQ-RB-07 - Does failed seed load still generate? -> No; false return skips `0x00598960` and later falls through common failure handling.` (evidence: `0x0068497A..0x00684995`; error path in `0x00684620`)
- `[RESOLVED] OQ-RB-08 - What generation call follows success? -> `0x00598960` is called with `ECX=0x00ABDFD8`, stack args `0,0`, then `ScenarioClass__Post_Map_Init(1)`.` (evidence: `0x00684980..0x00684990`)
- `[RESOLVED] OQ-RB-09 - Does launch generation repaint Choose Map preview? -> No; launch passes `param_2=0`, while `GenerateTerrainPreview/SendMessageA` blocks are guarded by nonzero `param_2`.` (evidence: `0x00684980..0x00684989`, `0x00598960`)
- `[RESOLVED] OQ-RB-10 - Does the branch replace the scenario filename with a generated map filename? -> No; it copies the original local filename back into `ScenarioClass+0x125C`.` (evidence: `0x00684995..0x006849BF`)
- `[RESOLVED] OQ-RB-11 - Does the normal branch differ? -> Non-SED calls `ScenarioClass__Read_Scenario_INI`, which opens the filename and copies it to `ScenarioClass+0x125C` before full init.` (evidence: `0x006849C9`; `0x00686730`)
- `[RESOLVED] OQ-RB-12 - What Rust surface currently fails this handoff? -> `load_map_by_name_or_path` treats requested map strings as real files; no `.SED` random-generation route exists.` (evidence: `src/app_list_maps.rs`, `src/app_init.rs`)
- `[DEFERRED] OQ-RB-13 - Which concrete vtable function is bound at `0x00ABDFD8 + 0x4`?` (category: bounded-cost-too-high; reason: wrapper dispatch and caller evidence are sufficient for launch contract, but the vtable table was not dumped by available read-only tools; next-step-if-pursued: resolve `vtable__MapSeedClass` data entries and decompile slot `+0x4`)
- `[DEFERRED] OQ-RB-14 - Exact random terrain formulas and seed field meanings beyond launch state.` (category: out-of-scope; reason: this report stops at branch handoff; next-step-if-pursued: dedicated RMG formula report over `0x00598960` callees)
- `[DEFERRED] OQ-RB-15 - Runtime failure frequency for malformed or missing `.SED` files.` (category: needs-runtime-debugger; reason: static branch behavior is verified, but stock failure frequency requires runtime/file experiments; next-step-if-pursued: run native with malformed custom `.sed` sentinel)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| `RandMap.Sed` reaches scenario load as the retained filename; `.SED` suffix, not a special index, selects random generation. | `0x0068465C..0x006846BE`; prior `0x005E7BF0` report | partially modeled sentinel, missing load branch | `src/skirmish_scenarios.rs`, `src/skirmish_launch.rs`, `src/app_init.rs`, `src/app_list_maps.rs` | Preserve `selected_map_file = "RandMap.Sed"` for session identity, but route `.sed`/random sentinel to a random-map generation path instead of normal file open. | Accept Create Random Map, press Start, and loading should not report "Map 'RandMap.Sed' not found". | Do not encode random map as `None`, a negative selected index, or a display-name-only selection. |
| Successful native random load calls `0x00597A10(filename)` before generation and aborts generation on failure. | `0x0068496B..0x0068497E`; `0x00597A10` | missing seed/options load concept | future random-map seed/session model | Add a seed/options object for generated maps and make generation depend on a validated random-map seed/options load. | Malformed/unsupported `.sed` random seed should fail before generated terrain is installed. | Do not silently fall back to a stock map or auto map when `.sed` seed load fails. |
| Launch generation calls `0x00598960(0,0)` and does not run the preview repaint path used by the random-map dialog. | `0x00684980..0x00684989`; `0x00598960` guarded preview blocks; `0x00596300` dialog passes `1, hwnd` | missing | app loading/generation path; `src/app_skirmish_shell_render.rs` only for preview surfaces | Keep launch-time random generation separate from Choose Map preview refresh; generation should populate game map state, not mutate modal preview UI. | Starting a random map should not depend on an existing preview texture/window handle. | Do not reuse the Choose Map preview-generation path as the authoritative game map loader. |
| After generation, native leaves `ScenarioClass+0x125C` as the original `.SED` filename and then runs `ScenarioClass__Post_Map_Init(1)`. | `0x0068498E..0x006849BF`; `0x00686890` | missing post-generation session identity | `src/app_init.rs`, `src/app_skirmish.rs`, snapshot/replay map-name surfaces if random maps are added | Generated game state may be in-memory while the scenario/session filename remains `RandMap.Sed`; post-generation spawn/session setup still runs. | Save/replay/session metadata for a random skirmish should retain `RandMap.Sed` or an explicit random-sentinel identity, not invent a fake loose-map path. | Do not write or require `RandMap.Map`/generated `.map` as the launch filename. |

### Negative Facts / Do Not Do

- Do not load `RandMap.Sed` through `load_map_by_name_or_path` as a normal concrete map file; native `.SED` detection routes away from `ScenarioClass__Read_Scenario_INI`. Evidence: `0x00684961..0x006849C9`.
- Do not replace `ScenarioClass+0x125C` with `RandMap.Map` or another generated filename after generation; native copies the original local filename back. Evidence: `0x00684995..0x006849BF`.
- Do not use a special negative index for launch; prior Choose Map accept commits the ordinary sentinel record index, and this branch only sees the filename. Evidence: prior `SKIRMISH_RANDMAP_SED_RANDOM_MAP_BEHAVIOR_GHIDRA_REPORT.md`; `0x00684620`.
- Do not run Choose Map preview repaint behavior during launch generation; native launch passes `0,0` to `0x00598960`. Evidence: `0x00684980..0x00684989`, guarded preview blocks in `0x00598960`.
- Do not treat every random-map-related generator function as proven formula parity here; only the immediate branch/filename/state handoff was drained. Evidence: coverage ledger defers `0x00598960` terrain internals.

### Remaining Uncertainty

- The concrete vtable slot function behind `0x00ABDFD8 + 0x4` was not resolved by a data-table dump in this read-only pass; the wrapper dispatch and caller contract are verified.
- Exact terrain formulas and seed field semantics inside `0x00598960` remain out of scope.
- Runtime behavior for malformed custom `.SED` files needs a debugger/file experiment if Rust wants exact error UX parity.

### Stale Docs / Follow-up Docs

- `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_SELECTED_MAP_TOKEN_LOAD_CONSUMER_GHIDRA_REPORT.md`: replace OQ-9 wording with:
  > For the random sentinel, the selected-record loader still copies record `+0x58` (`RandMap.Sed`) into `ScenarioClass+0x125C`. `ScenarioClass__Read_Scenario @ 0x00684620` detects the `.SED` suffix, sets `ScenarioClass+0x34BD = 1`, calls `0x00597A10` with `ECX=0x00ABDFD8` and the local filename, then on success calls `0x00598960(0,0)` and `ScenarioClass__Post_Map_Init(1)`. After the branch it copies the original local filename back into `ScenarioClass+0x125C`; no generated map filename replaces `RandMap.Sed`.
- `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_RANDMAP_SED_RANDOM_MAP_BEHAVIOR_GHIDRA_REPORT.md`: replace "launch detects the `.SED` filename suffix and runs the random-map generation path" with:
  > Launch detects the `.SED` suffix, calls the seed-load wrapper `0x00597A10` with the local filename, calls `0x00598960(0,0)` only on success, then retains the original `.SED` filename in `ScenarioClass+0x125C` while generated map state is in memory.

## Sources

- Ghidra read-only decompile / assembly: `0x00684620`, `0x00684961..0x006849BF`, `0x00597A10`, `0x00598960`, `0x00686890`, `0x00686730`, `0x00596300`, `0x00595680`, `0x00597260`, `0x00597380`, `0x00597430`, `0x005975E0`, `0x005981F0`, `0x00599650`.
- Prior docs: `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_SELECTED_MAP_TOKEN_LOAD_CONSUMER_GHIDRA_REPORT.md`; `C:/Users/enok/Documents/ra2-rust-game-docs/skirmish-ui/SKIRMISH_RANDMAP_SED_RANDOM_MAP_BEHAVIOR_GHIDRA_REPORT.md`.
- Local INI: `C:/Users/enok/Documents/ra2-rust-game/ini/mpmodesmd.ini`; `C:/Users/enok/Documents/ra2-rust-game/ini/rulesmd.ini`.
- Rust scan: `C:/Users/enok/Documents/ra2-rust-game/src/skirmish_scenarios.rs`; `src/ui/skirmish_shell/state.rs`; `src/skirmish_launch.rs`; `src/app.rs`; `src/app_init.rs`; `src/app_list_maps.rs`.
