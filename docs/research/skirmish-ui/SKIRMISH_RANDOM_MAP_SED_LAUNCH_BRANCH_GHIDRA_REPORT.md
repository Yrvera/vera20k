# Skirmish Random Map `.SED` Launch Branch - Ghidra Research Report

**Address(es):** `0x00684620`, `0x00597A10`, `0x00598960`, `0x00686730`, `0x00686890`, `0x00683AB0`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** downstream launch/start-game handling after the shell-selected scenario filename reaches `ScenarioClass__Read_Scenario`, specifically filenames whose final four characters case-insensitively compare equal to `.SED`, including the standard `RandMap.Sed` sentinel.  
**Non-Scope:** Create Random Map setup dialog controls, random-map terrain formulas, exact candidate scoring, `RandMap.img` preview contents, Choose Map modal paint, malformed external `.SED` runtime UX beyond static failure branches, and unrelated Random side/color/start resolution.  
**Confidence:** High for suffix predicate, active-YR liveness, seed-load wrapper dispatch shape, success/failure call order, generator mode arguments, normal `.MPR/.YRM` branch separation, and Rust launch delta. Medium for the concrete vtable target behind `MapSeedClass+0x4` because this read-only pass verified vtable bytes and wrapper dispatch but did not create/decompile a missing function at `0x00597A30`.  
**Active in YR:** Conditional. Active in standard YR offline Skirmish after the player accepts the random-map sentinel (`RandMap.Sed`) from a random-map-enabled mode. Inactive for ordinary non-`.SED` scenario filenames.

## 0. Investigation Gate

- Target question: when a selected Skirmish scenario filename ends in `.SED`, how does standard YR detect random-map launch, what load/generation functions run, how does that differ from normal scenario INI loading, and what must Rust implement?
- Non-goals: do not reopen setup dialog UI, `0x583` accept/cancel internals, `RandMap.img`, full RMG formulas, or side/color/start launch gaps.
- Evidence needed to mark COMPLETE: decompile plus assembly for `ScenarioClass__Read_Scenario @ 0x00684620`; xref/caller evidence from `ScenarioClass__Start_Scenario`; decompile plus assembly for `0x00597A10`; caller/callee evidence for `0x00598960`; normal branch evidence for `0x00686730`; Rust surface scan.
- Stop conditions: stop after branch selection, seed-load/generation call order, filename retention, and Rust handoff are implementation-ready; defer full generator internals and malformed-file UX.

## 1. Overview

Native YR does not special-case `RandMap.Sed` by exact filename at launch. `ScenarioClass__Read_Scenario @ 0x00684620` copies the selected filename to a local buffer, compares the suffix with `.SED`, writes `ScenarioClass+0x34BD`, and then chooses between the random-map seed/generator path and the normal scenario INI path.

On the random path, it calls seed-load wrapper `0x00597A10` with `ECX=0x00ABDFD8` and the local filename pointer. Only if that wrapper returns nonzero does it call the generator `0x00598960` with `ECX=0x00ABDFD8` and stack args `0,0`, then `ScenarioClass__Post_Map_Init(1)`. After the branch, it copies the original `.SED` filename back into `ScenarioClass+0x125C`; no generated `.map` filename replaces `RandMap.Sed`.

## 2. Key State / Offsets

| State | Meaning in this slice | Evidence | Active in YR |
|---|---|---|---|
| `ScenarioClass+0x125C` | retained scenario filename buffer; remains the original `.SED` token after generation | `0x00683AB0` copy-in; `0x00684995..0x006849BF` copy-back | Yes / Conditional |
| `ScenarioClass+0x34BD` | random-map flag set from `.SED` suffix compare | `0x00684694..0x006846BE`, read at `0x00684961` | Yes / Conditional |
| `ScenarioClass+0x3598` | scenario-load-in-progress byte; set before branch, read by `0x00598960` to choose loading-progress behavior | write `0x00684675`; reads in `0x00598960` | Yes |
| `0x00ABDFD8` | `MapSeedClass`/random seed object passed as `this` to seed-load wrapper and generator | `0x0068496F`, `0x00684984`; vtable bytes at `0x007ED8E4` | Yes / Conditional |
| `MapSeedClass+0x74` | seed consumed immediately by generator RNG setup via `FUN_0065C6D0(*(this+0x74))` | `0x0059897B..0x00598985` | Yes / Conditional |
| `MapSeedClass+0x50` | `.SED [RandomMap] NumPlayers`; downstream start metadata loop bound, not the suffix selector | prior `RMG_START_GENERATION_00594B50_005A1FB0...`; reader assembly `0x00597B42..0x00597B5C` | Yes / Conditional |

## 3. Core Logic

### 3.1 Start-to-read liveness

Active in YR: Yes. `ScenarioClass__Start_Scenario @ 0x00683AB0` copies its scenario filename argument into `ScenarioClass+0x125C`, logs it, constructs a `CCFileClass`/`SHAPipe` for intro/briefing metadata, then calls `ScenarioClass__Read_Scenario`. `get_function_xrefs` shows the read function is called from `0x00683D21` in `ScenarioClass__Start_Scenario`.

Evidence: decompile `0x00683AB0`; xref `From 00683d21 in ScenarioClass__Start_Scenario [UNCONDITIONAL_CALL]`; read decompile `0x00684620`.

Active in YR: Conditional. Existing Choose Map/Create Random Map reports verify the accepted `0x583` path upserts an ordinary selected scenario record with filename `RandMap.Sed`, and accepted selection is passed forward as a normal scenario filename. This report's launch slice verifies what happens once that filename reaches `0x00684620`.

Evidence: prior docs `SKIRMISH_CREATE_RANDOM_MAP_0X583_IMPLEMENTATION_CONTRACT_GHIDRA_REPORT.md`, `SKIRMISH_RANDMAP_SED_WRITER_00597730_LAYOUT_GHIDRA_REPORT.md`; current `0x00684620` branch evidence below.

### 3.2 `.SED` predicate

Active in YR: Yes / Conditional. `0x00684620` copies the input string to a local 260-byte buffer, then compares the filename suffix against literal `.SED` at `0x0083DA88`. The compare helper is `0x007C8D20`, which performs a case-insensitive byte string compare and returns `0` on equality.

Evidence:

- assembly `0x0068465C PUSH 0x83DA88`, `0x00684691 ADD ECX,EDX`, `0x00684693 PUSH ECX`, `0x00684694 CALL 0x007C8D20`;
- memory `0x0083DA88` decodes to string `.SED`;
- decompile `0x007C8D20` shows case-insensitive comparison and equality return `0`.

Active in YR: Yes / Conditional. If the compare returns equal, native writes `ScenarioClass+0x34BD = 1`; otherwise it writes `0`. The later branch reads this byte instead of comparing the filename again.

Evidence: assembly `0x0068469C TEST EAX,EAX`, `0x0068469E JNZ 0x006846B3`, `0x006846AA MOV byte ptr [EAX+0x34BD],0x1`, `0x006846BE MOV byte ptr [ECX+0x34BD],0x0`; read at `0x00684961`.

### 3.3 Random branch call order

Active in YR: Conditional. When `ScenarioClass+0x34BD != 0`, `0x00684620` loads `ECX=0x00ABDFD8`, pushes `&local_filename`, and calls `0x00597A10`.

Evidence: assembly `0x00684961 MOV AL,[ECX+0x34BD]`, `0x00684967 TEST AL,AL`, `0x00684969 JZ 0x006849C3`, `0x0068496B LEA EDX,[ESP+0x40]`, `0x0068496F MOV ECX,0xABDFD8`, `0x00684974 PUSH EDX`, `0x00684975 CALL 0x00597A10`.

Active in YR: Conditional. `0x00597A10` is a wrapper. With a non-null filename argument, it calls `this->vtable+0x4` and returns that byte result; with a null argument it calls `0x005587F0` and returns whether that succeeded. The launch caller always passes a non-null local filename, so the null/default helper is not used for standard `RandMap.Sed` launch.

Evidence: decompile `0x00597A10`; assembly `0x00597A10 MOV EAX,[ESP+4]`, `0x00597A14 TEST EAX,EAX`, `0x00597A16 JZ 0x00597A21`, `0x00597A18 MOV EDX,[ECX]`, `0x00597A1A PUSH EAX`, `0x00597A1B CALL dword ptr [EDX+0x4]`, `0x00597A1E RET 0x4`.

Active in YR: Conditional. The `MapSeedClass` vtable bytes at `0x007ED8E4` are `70 C2 5A 00 30 7A 59 00 60 77 59 00 50 7D 59 00`, so slot `+0x4` points to `0x00597A30` and slot `+0x8` points to writer `0x00597760`. `0x00597A30` was not a defined function in this read-only Ghidra project, but disassembly at that address shows a reader body that uses `[RandomMap]` strings and reads keys including `NumPlayers`, `Seed`, `MapType`, `Theater`, and others.

Evidence: `read_memory 0x007ED8E4`; disassembly context `0x00597A30`; key reads around `0x00597B42..0x00597CB2`; prior layout report drains the writer/reader layout.

### 3.4 Success and failure handling

Active in YR: Conditional. If seed-load wrapper returns zero, generation is skipped. The function still copies the local filename back into `ScenarioClass+0x125C`, then reaches the common error path because the branch result byte remains false.

Evidence: assembly `0x0068497A MOV BL,AL`, `0x0068497C TEST BL,BL`, `0x0068497E JZ 0x00684995`; common error test at `0x006849EC TEST BL,BL` followed by error dialog/log path.

Active in YR: Conditional. If seed-load succeeds, native calls the generator with `ECX=0x00ABDFD8` and two zero stack arguments, then calls `ScenarioClass__Post_Map_Init` with `CL=1`.

Evidence: assembly `0x00684980 PUSH 0x0`, `0x00684982 PUSH 0x0`, `0x00684984 MOV ECX,0xABDFD8`, `0x00684989 CALL 0x00598960`, `0x0068498E MOV CL,0x1`, `0x00684990 CALL 0x00686890`.

Active in YR: Conditional. After either random success or random failure, the branch copies the original local filename buffer into `ScenarioClass+0x125C`. The copied token remains `RandMap.Sed` for the standard sentinel. No generated `.map` path is written here.

Evidence: assembly `0x00684995 MOV EAX,[0x00A8B230]`, `0x006849A1 LEA EDX,[EAX+0x125C]`, `0x006849B3 MOV EDI,EDX`, `0x006849B8 MOVSD.REP`, `0x006849BF MOVSB.REP`.

### 3.5 Generator mode: launch vs preview

Active in YR: Conditional. `0x00598960` has preview/UI repaint blocks guarded by `(char)param_2 != 0`, calling `GenerateTerrainPreview()` and `SendMessageA(hwnd, WM_PAINT, 0, 0)`. The scenario-load branch passes stack args `0,0`, so launch generation does not run those guarded preview repaint blocks.

Evidence: launch caller `0x00684980..0x00684989`; decompile `0x00598960`; dialog caller `0x00596300` calls `FUN_00598960(1,param_1)` and separately calls `GenerateTerrainPreview`.

Active in YR: Conditional. Because `ScenarioClass+0x3598` is set to `1` before the branch, `0x00598960` uses scenario-loading progress callbacks (`FUN_0069AE90(0x9A)`, `0x9F`, `0xA4`, ..., `199`) rather than dialog progress helper `FUN_00643C50` in the launch path.

Evidence: write `0x00684675`; `0x00598960` repeated reads of `ScenarioClass+0x3598`; caller path from `0x00684620`.

### 3.6 Normal non-`.SED` branch

Active in YR: Yes for ordinary selected Skirmish maps. If `ScenarioClass+0x34BD == 0`, native calls `ScenarioClass__Read_Scenario_INI @ 0x00686730`. That function constructs a `CCFileClass` for the same filename, opens it through `SHAPipe`, copies the filename into `ScenarioClass+0x125C`, and calls `ScenarioClass__Full_Init`. The random `.SED` branch routes away from this normal scenario INI read.

Evidence: assembly `0x006849C3 MOV DL,0x1`, `0x006849C5 LEA ECX,[ESP+0x40]`, `0x006849C9 CALL 0x00686730`; decompile `0x00686730`.

## 4. INI / Data Inputs

| Source | Role | Evidence | Active in YR |
|---|---|---|---|
| `ini/mpmodesmd.ini:[Battle]` entry `1=..., standard, true` | mode allows random maps and can expose the sentinel after accepted setup | local INI; prior mode/source docs | Yes |
| `ini/mpmodesmd.ini:[FreeForAll]` entry `2=..., standard, true` | mode allows random maps | local INI | Conditional by selected mode |
| `ini/mpmodesmd.ini` entries with fifth field `false` | random sentinel should be filtered from those modes | local INI; prior mode parser docs | Conditional |
| `.SED [RandomMap]` keys | seed/options loaded by `0x00597A10` vtable `+0x4` before generator | vtable bytes; disassembly `0x00597A30`; prior layout report | Conditional |
| `.MPR/.YRM/.MAP` scenario INI | normal maps are opened through `ScenarioClass__Read_Scenario_INI`, not the random branch | `0x006849C9`, `0x00686730` | Yes for ordinary maps |

## 5. Integration Points

| Integration | Verified behavior | Evidence | Active in YR |
|---|---|---|---|
| Start scenario entry | `ScenarioClass__Start_Scenario` forwards selected filename to `ScenarioClass__Read_Scenario` | `0x00683AB0`; xref `0x00683D21 -> 0x00684620` | Yes |
| Suffix selector | final `.SED` suffix sets `ScenarioClass+0x34BD=1` | `0x0068465C..0x006846BE` | Conditional |
| Seed load | random branch calls `0x00597A10` with `ECX=0x00ABDFD8`, filename arg non-null | `0x0068496B..0x00684975`; `0x00597A10` | Conditional |
| Generation | seed-load success calls `0x00598960(0,0)` and `ScenarioClass__Post_Map_Init(1)` | `0x0068497C..0x00684990` | Conditional |
| Failure | seed-load false skips generator and reaches common read-scenario failure path | `0x0068497A..0x00684995`; `0x006849EC` | Conditional malformed/missing `.SED` |
| Filename retention | branch copies original local filename back into `ScenarioClass+0x125C` | `0x00684995..0x006849BF` | Conditional |
| Normal maps | non-`.SED` maps call `ScenarioClass__Read_Scenario_INI` | `0x006849C3..0x006849C9`, `0x00686730` | Yes |

## 6. Current Rust Implementation Status

| Area | Current Rust status | Evidence |
|---|---|---|
| scenario record model | `SkirmishScenarioRecord::random_map_sentinel` now uses `file_name = "RandMap.Sed"`, `kind = RandomMapSentinel`, `min_players = Some(2)`, `max_players = Some(4)`, `official = true` | `src/skirmish_scenarios.rs` |
| launch session | `SkirmishLaunchSession.selected_map_file` carries the selected record filename; shell launch stores `selected_map.file_name.clone()` | `src/skirmish_launch.rs`; `src/ui/skirmish_shell/state/launch.rs` |
| loading request | native selected Skirmish loading passes `selected_map_file` into `load_map_initial_with_assets` | `src/app_loading.rs` |
| map loader | `load_map_initial_with_assets` and `load_map_by_name_or_path_with_assets` try concrete map files and asset candidates; candidates omit `.sed` and no random generation path exists | `src/app_init.rs`; `src/app_list_maps.rs` |
| generated map state | no Rust random-map generator, no `.SED` parser/seed model, no generated waypoint/map metadata handoff exists | focused `rg`; codegraph context |
| downstream spawn | `seed_skirmish_opening_if_needed` and launch apply logic consume `MapFile.waypoints`; no generated random-map waypoints can reach spawn setup yet | `src/app_skirmish.rs` |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `ScenarioClass__Start_Scenario -> Read_Scenario` liveness | verified | decompile `0x00683AB0`; xref `0x00683D21` | none |
| `.SED` suffix compare | verified | decompile/assembly `0x0068465C..0x006846BE`; `.SED` string at `0x0083DA88`; helper `0x007C8D20` | none |
| random flag write/read | verified | writes `0x006846AA/0x006846BE`; read `0x00684961` | none |
| `0x00597A10` wrapper dispatch shape | verified | decompile/assembly `0x00597A10..0x00597A2B` | concrete `0x00597A30` decompile unavailable without defining missing function; prior layout report covers it |
| `MapSeedClass` vtable slot bytes | verified | memory `0x007ED8E4 = 70 C2 5A 00 30 7A 59 00 60 77 59 00 50 7D 59 00` | none for launch contract |
| generator launch call | verified | assembly `0x00684980..0x00684990`; decompile `0x00598960` | full generator formulas out of scope |
| preview-vs-launch argument distinction | verified | launch `0,0`; dialog `0x00596300` passes `1, hwnd`; guarded blocks in `0x00598960` | modal visual behavior out of scope |
| filename copy-back | verified | assembly `0x00684995..0x006849BF` | none |
| normal scenario INI branch | verified | `0x006849C9`, decompile `0x00686730` | loose/MIX file precedence covered by separate report |
| Rust launch delta | verified | codegraph + file reads listed above | implementation not performed |
| malformed external `.SED` exact UX | deferred | static failure path verified | runtime debugger/file experiment |
| full random terrain formulas | deferred | `0x00598960` callees identified | dedicated RMG formula investigations |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-01 - Is this code active in standard YR Skirmish? -> Yes, conditionally when the accepted selected scenario filename ends in `.SED`, including standard `RandMap.Sed`.` (evidence: `0x00683AB0`; `0x00684620`; prior `0x583` reports)
- `[RESOLVED] OQ-02 - What is the branch predicate? -> case-insensitive suffix compare with literal `.SED`, not exact `RandMap.Sed`.` (evidence: `0x0068465C..0x006846BE`; `0x0083DA88`; `0x007C8D20`)
- `[RESOLVED] OQ-03 - Where is random state stored? -> `ScenarioClass+0x34BD` is written `1` on suffix match and `0` otherwise.` (evidence: `0x006846AA`; `0x006846BE`)
- `[RESOLVED] OQ-04 - What happens first on random branch? -> seed-load wrapper `0x00597A10` receives `ECX=0x00ABDFD8` and `&local_filename`.` (evidence: `0x0068496B..0x00684975`)
- `[RESOLVED] OQ-05 - Does launch use the null/default seed-load wrapper path? -> No, the caller passes a non-null filename.` (evidence: `0x00597A10..0x00597A21`; `0x00684974`)
- `[RESOLVED] OQ-06 - Does generation run after failed seed load? -> No, zero return skips `0x00598960`.` (evidence: `0x0068497A..0x00684995`)
- `[RESOLVED] OQ-07 - What generator arguments does launch use? -> `ECX=0x00ABDFD8`, stack args `0,0`.` (evidence: `0x00684980..0x00684989`)
- `[RESOLVED] OQ-08 - Does launch call post-map init? -> Yes, only after successful generation, with `CL=1`.` (evidence: `0x0068498E..0x00684990`; decompile `0x00686890`)
- `[RESOLVED] OQ-09 - Does launch update Choose Map preview? -> No; launch passes `param_2=0`, while preview repaint blocks require nonzero `param_2`.` (evidence: `0x00684980..0x00684989`; `0x00598960`; dialog `0x00596300`)
- `[RESOLVED] OQ-10 - Does native replace `RandMap.Sed` with a generated filename? -> No; it copies the original local filename back into `ScenarioClass+0x125C`.` (evidence: `0x00684995..0x006849BF`)
- `[RESOLVED] OQ-11 - How do normal `.MPR/.YRM/.MAP` maps load? -> non-`.SED` branch calls `ScenarioClass__Read_Scenario_INI`, which opens the scenario INI and full-inits it.` (evidence: `0x006849C9`; `0x00686730`)
- `[RESOLVED] OQ-12 - Does Rust already have a launch branch for `.SED`? -> No; Rust preserves a sentinel filename but routes loading through concrete map lookup/generic `MapFile` parsing.` (evidence: `src/app_loading.rs`; `src/app_init.rs`; `src/app_list_maps.rs`)
- `[RESOLVED] OQ-13 - Null pointer edge case for `0x00597A10`? -> wrapper handles null by calling `0x005587F0`, but standard launch does not pass null.` (evidence: `0x00597A14..0x00597A2B`; caller `0x00684974`)
- `[RESOLVED] OQ-14 - Empty/short filename edge? -> static predicate still computes a suffix pointer and compares; exact crash/underflow UX for malicious direct calls is not a standard shell path.` (evidence: `0x0068465C..0x00684694`; standard selected-record handoff prevents empty sentinel)
- `[RESOLVED] OQ-15 - Tick-cycle integration? -> this runs during scenario load between Start_Scenario setup and post-load finalization, not a game tick update; `ScenarioClass+0x3598` causes loading-progress callbacks.` (evidence: `0x00683AB0`; `0x00684675`; `0x00598960`)
- `[RESOLVED] OQ-16 - TS legacy filter? -> not TS-only; branch is reachable from standard YR Skirmish random-map sentinel, though conditional on user path/mode.` (evidence: prior `0x583` liveness docs; current `0x00684620`)
- `[DEFERRED] OQ-17 - Exact generated terrain formulas and RNG consumption inside all `0x00598960` callees.` (category: out-of-scope; reason: this slice targets launch branch handoff; next-step-if-pursued: random-map generator formula swarm)
- `[DEFERRED] OQ-18 - Malformed external `.SED` user-facing error for every bad field.` (category: needs-runtime-debugger; reason: static failure and no reader-side clamp are known, but exact UX needs native file experiments; next-step-if-pursued: launch crafted `.SED` files)
- `[DEFERRED] OQ-19 - Exact concrete decompile of `0x00597A30` in this Ghidra project.` (category: bounded-cost-too-high; reason: function is not defined and mutating `create_function` is forbidden; prior layout report plus current vtable/disassembly cover launch contract; next-step-if-pursued: read-only project with function already defined or approved mutation in separate session)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| `.SED` suffix, not a special exact `RandMap.Sed` equality or negative index, selects random-map loading. | `0x0068465C..0x006846BE`; `0x007C8D20`; prior selected-record docs | sentinel exists, but no load branch | `src/skirmish_scenarios.rs`; `src/skirmish_launch.rs`; `src/app_loading.rs`; `src/app_init.rs` | Preserve the selected filename string and route case-insensitive `.sed` suffixes to a random-map seed/generation path before concrete map parsing. | Accept/create random map, press Start, and loading does not report `Map 'RandMap.Sed' not found`; proposed test `skirmish_randmap_sed_suffix_routes_to_random_generation` | Do not encode random map as `None`, display name only, or special negative selected index. |
| Native calls seed/options load before generation and skips generation on false. | `0x0068496B..0x0068497E`; `0x00597A10..0x00597A2B`; vtable bytes `0x007ED8E4` | no `.SED` parser/seed object | new random-map seed/options model; `src/app_init.rs`; `src/app_list_maps.rs` or map-loader layer | Add a `[RandomMap]` seed/options load step for `.SED` and make generated-map install conditional on successful seed load. | A malformed/missing `.SED` fails before terrain/spawn state is installed; proposed test `skirmish_randmap_sed_load_failure_skips_generation` | Do not silently fall back to a stock map or auto-generate with default seed when seed load fails. |
| Launch generation calls `0x00598960(0,0)` and does not use dialog preview repaint mode. | `0x00684980..0x00684989`; `0x00598960`; dialog caller `0x00596300` | no generator; preview and launch not separated | future random-map generator entry; shell preview renderer remains separate | Keep launch generation as gameplay map-state generation with preview flag false/window null; preview generation must not be the authoritative game loader. | Starting a random map works without an open Choose Map dialog or preview texture; proposed test `skirmish_randmap_launch_generation_does_not_require_preview_surface` | Do not use `RandMap.img` or modal preview bytes as gameplay terrain. |
| Native retains original `.SED` filename in `ScenarioClass+0x125C` after generation, then continues normal post-map init/spawn setup. | `0x0068498E..0x006849BF`; decompile `0x00686890` | Rust has no generated map session identity or generated waypoint handoff | `src/app_loading.rs`; `src/app_init.rs`; `src/app_skirmish.rs`; save/replay/session metadata surfaces | Generated map state may be in memory while the selected/session map identity remains `RandMap.Sed`; generated waypoints/map data must feed normal Skirmish spawn setup afterward. | Session metadata retains `RandMap.Sed` while spawned players use generated waypoint slots; proposed test `skirmish_randmap_launch_retains_sed_identity_after_generation` | Do not invent or require `RandMap.map`/generated loose file as the launch filename. |
| Ordinary non-`.SED` maps route to `ScenarioClass__Read_Scenario_INI`; `.SED` must not be parsed as ordinary map INI. | `0x006849C3..0x006849C9`; decompile `0x00686730` | current loader treats requested map strings as concrete map candidates | `src/app_list_maps.rs`; `src/app_init.rs` | Split `.sed` random seed loading before ordinary `.map/.mpr/.yrm/.mmx/.yro` map lookup. | Ordinary selected `*.yrm` still uses concrete map loader, while `RandMap.Sed` does not; proposed test `skirmish_sed_branch_does_not_use_concrete_map_loader` | Do not add `.sed` to `asset_map_candidates` as though it were a normal `MapFile`. |

## 10. Negative Facts / Do Not Do

- Do not treat Create Random Map launch as TS-only legacy. Active in YR: Conditional yes through standard Skirmish sentinel. Evidence: prior `0x583` reports plus current `0x00684620` launch branch.
- Do not special-case only exact `RandMap.Sed` at the branch selector. Native compares suffix `.SED` case-insensitively. Evidence: `0x0068465C..0x006846BE`; `0x007C8D20`.
- Do not parse `RandMap.Sed` through the normal map/INI `MapFile` loader. Native random branch skips `ScenarioClass__Read_Scenario_INI`. Evidence: `0x00684961..0x006849C9`.
- Do not replace the session filename with a generated `.map` filename after generation. Native copies the original local filename back into `ScenarioClass+0x125C`. Evidence: `0x00684995..0x006849BF`.
- Do not use the Choose Map preview/repaint path as launch generation. Native launch passes `0,0` to `0x00598960`; dialog preview passes nonzero mode/window. Evidence: `0x00684980..0x00684989`; `0x00596300`; `0x00598960`.
- Do not assume the seed reader clamps external `.SED` fields on launch. Prior layout report found no reader-side normalizer call before `0x00598960`; exact bad-file UX remains runtime-deferred.

## 11. Remaining Uncertainty

- Exact generated terrain formulas and RNG consumption inside every `0x00598960` callee remain out of scope.
- Exact runtime UX for malformed external `.SED` files needs native file/debugger experiments.
- `0x00597A30` could not be decompiled directly in this read-only project because it is not defined as a function; vtable bytes, disassembly, and the prior layout report verify the launch-facing reader contract.

## 12. Stale Docs / Replacement Wording

No contradiction found with the current high-confidence docs. If a summary says only "launch detects `RandMap.Sed` and generates a random map", replace it with:

> Launch detects a case-insensitive `.SED` suffix in `ScenarioClass__Read_Scenario`, sets `ScenarioClass+0x34BD`, calls seed-load wrapper `0x00597A10` with `ECX=0x00ABDFD8` and the local filename, calls `0x00598960(0,0)` plus `ScenarioClass__Post_Map_Init(1)` only on seed-load success, and then retains the original `.SED` filename in `ScenarioClass+0x125C`.

## Sources

- Ghidra read-only decompile/assembly: `0x00683AB0`, `0x00684620`, `0x0068465C..0x006846BE`, `0x00684961..0x006849C9`, `0x00597A10`, `0x00597A30` disassembly, `0x00598960`, `0x00596300`, `0x00686730`, `0x00686890`, `0x007C8D20`.
- Ghidra data: string `.SED` at `0x0083DA88`; vtable bytes at `0x007ED8E4`.
- Prior research: `docs/research/skirmish-ui/SKIRMISH_RANDOM_MAP_BRANCH_AFTER_SELECTED_MAP_LOAD_GHIDRA_REPORT.md`; `docs/research/skirmish-ui/SKIRMISH_RANDMAP_SED_WRITER_00597730_LAYOUT_GHIDRA_REPORT.md`; `docs/research/skirmish-ui/RMG_START_GENERATION_00594B50_005A1FB0_GHIDRA_REPORT.md`; `docs/research/skirmish-ui/SKIRMISH_CREATE_RANDOM_MAP_0X583_IMPLEMENTATION_CONTRACT_GHIDRA_REPORT.md`.
- INI: `ini/mpmodesmd.ini`.
- Rust scan: `src/skirmish_scenarios.rs`; `src/skirmish_launch.rs`; `src/ui/skirmish_shell/state/launch.rs`; `src/app_loading.rs`; `src/app_init.rs`; `src/app_list_maps.rs`; `src/app_skirmish.rs`.
