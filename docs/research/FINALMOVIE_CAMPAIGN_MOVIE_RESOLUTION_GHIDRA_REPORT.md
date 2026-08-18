# FinalMovie Campaign Movie Resolution - Ghidra Research Report

**Address(es):** `0x0052CBA0`, `0x0046CE10`, `0x0046CCD0`, `0x004757D0`, `0x0048DF30`, `0x00685670`, `0x005BF260`, `0x005C0640`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** `battle.ini` / `battlemd.ini` `FinalMovie=` parsing into `CampaignClass`, and the mission-end consumer that decides whether to play that campaign final movie.
**Non-Scope:** main-menu RA2TS Bink loop/vtable; Movies & Credits picker UI; exact Bink frame loop; complete campaign progression and score-screen internals; all `[Basic] Intro/Brief/Win/Lose/PostScore/PreMapSelect` movie consumers except where they share the same movie-index resolver.
**Confidence:** High for reader/consumer chain and file-extension fallback; Medium for stock-content liveness beyond repo INI copies because retail campaign map contents were not extracted in this slot.
**Active in YR:** Conditional. The parser and end-of-mission consumer are live in standard YR, but stock `ini/battlemd.ini` and `ini/battle.ini` contain only blank `FinalMovie=` values, so stock campaign records resolve `FinalMovie` to `-1` unless a `BATTLEMD*.INI` addon supplies a value.

## 0. Working Notes Required By Slot

- Target question: How does `FinalMovie=` in `battle.ini` / `battlemd.ini` resolve to the movie played at mission/campaign end?
- Non-goals: Do not reinvestigate the main-menu Bink loop, the Movies & Credits picker, full Bink audio/video decoding, or unrelated score-screen presentation.
- Evidence needed to mark COMPLETE: INI source defaults; binary loader for `BATTLEMD*.INI`; `FinalMovie` binary reader; movie-name-to-index resolver; end-of-mission consumer; file extension and BIK/VQA fallback used by that consumer; Rust-facing delta.
- Stop conditions: Stop after the `FinalMovie=` index reaches `FUN_005BF260` or is proven unused; record any broader campaign-progression or asset-runtime questions as deferred.

## 1. Overview

`FinalMovie=` is not stored as a filename on `CampaignClass`. The campaign reader converts it immediately into an integer index into the global `[Movies]` table populated from `art(md).ini`. At mission end, the scenario shutdown path checks `[Basic] EndOfGame=yes`; if the current scenario has a valid campaign index, it loads `CampaignClass+0x29C` and calls the generic movie-index playback wrapper.

For standard YR content in this repo, every `FinalMovie=` in `battlemd.ini` and `battle.ini` is blank. That means the mechanism is live, but the stock data path does not request a final campaign movie through this key.

## 2. Class Layout / Key Offsets

| Object | Offset | Purpose | Evidence | Active in YR |
|---|---:|---|---|---|
| `CampaignClass` | `+0x24` | internal campaign section/name inherited from `AbstractTypeClass` | `0x0046CE10`, `0x0046CCD0` | Yes |
| `CampaignClass` | `+0x98` | `CD=` integer | `0x0046CCD0`, `CCINIClass__ReadInt(..., "CD", old)` | Yes |
| `CampaignClass` | `+0x9C` | `Scenario=` string, length `0x200`, normalized by `0x007DCFC4` | `0x0046CCD0` | Yes |
| `CampaignClass` | `+0x29C` | `FinalMovie=` as `[Movies]` index, default/invalid `-1` | `0x0046CCD0`, `0x004757D0`, `0x006859E3` | Conditional |
| `CampaignClass` | `+0x2A0` | localized or raw description buffer, length `0x80` | `0x0046CCD0`, `0x0052F02B` | Yes |
| `ScenarioClass` | `+0x34AC` | `[Basic] EndOfGame` bool | `0x0068A11F`, `0x006859BF` | Conditional |
| `ScenarioClass` | `+0x34AF` | `[Basic] OneTimeOnly` bool; suppresses the end-game movie/score branch when set | `0x0068A1F3`, `0x006859B1` | Conditional |
| `ScenarioClass` | `+0x34CC` | current campaign index | `0x00683B0A`, `0x006859C9` | Conditional |
| global movie table | `DAT_00ABF394` | array of movie-name pointers from `[Movies]` | `0x00674550`, `0x0048DF30`, `0x005BF260` | Yes |
| global movie count | `DAT_00ABF3A0` | count used for index bounds | `0x00674550`, `0x005BF260` | Yes |

## 3. Core Logic

### 3.1 Campaign list load

`CDFileClass__Constructor @ 0x0052CBA0` searches for `BATTLEMD*.INI` (`0x008261A8`). For every matching non-directory file it constructs a `CCINIClass`, loads the file, and calls `CampaignClass__Constructor @ 0x0046CE10`. If no exact `BATTLEMD.INI` (`0x00826198`) was seen in the wildcard pass, it explicitly loads `BATTLEMD.INI` and calls the same constructor.

Active in YR: Yes. The YR executable uses `BATTLEMD*.INI`, not `BATTLE*.INI`, in this active loader. Base `battle.ini` is useful for comparison but not the primary YR loader in this path.

### 3.2 Campaign objects are created from `[Battles]`

`CampaignClass__Constructor @ 0x0046CE10` counts entries in `[Battles]` (`0x0081B1D8`), reads each value into a 32-byte local buffer, deduplicates against existing campaign objects by comparing `Campaign+0x24`, allocates a `0x3A0` object when needed, initializes `Campaign+0x98 = -1` and `Campaign+0x29C = -1`, then virtual-calls slot `+0x64`, which resolves to the campaign INI reader at `0x0046CCD0`.

Active in YR: Yes. Standard `ini/battlemd.ini` lists `ALL1`, `SOV1`, and debug mission sections after them; all are parsed unless filtered by later UI logic.

### 3.3 `FinalMovie=` reader

`FUN_0046CCD0 @ 0x0046CCD0` first validates the section by calling `0x00410A60`. It then reads:

- `CD=` into `Campaign+0x98`.
- `FinalMovie=` through `FUN_004757D0(section, "FinalMovie", old Campaign+0x29C)`, storing the returned movie index to `Campaign+0x29C`.
- `Scenario=` into `Campaign+0x9C`, max `0x200`, then calls `0x007DCFC4` to normalize the scenario string.
- `DebugOnly=`; if false, it reads/localizes `Description=` into `Campaign+0x2A0`, max `0x80`; if true, it appends `" (for debug testing)"` to the raw description and calls `FUN_00735060(-1)`.

Active in YR: Yes for the reader. Conditional for non-blank `FinalMovie`, because stock `battlemd.ini` and `battle.ini` values are blank.

### 3.4 Movie-name-to-index resolver

`FUN_004757D0 @ 0x004757D0` reads a string from the current section/key with default empty string (`DAT_00889F64`) into a 128-byte local buffer. If `CCINIClass__ReadString` reports a nonzero read and `FUN_0048DF30(name)` finds the movie in `DAT_00ABF394`, the found index is returned. Otherwise it returns the previous/default index supplied by the caller.

`FUN_0048DF30 @ 0x0048DF30` returns `-1` for null input or empty string. For non-empty input, it loops `0 <= index < DAT_00ABF3A0` and compares the input against `DAT_00ABF394[index]` with `FUN_007C8D20`, a case-insensitive string compare. Exact spelling case in INI therefore does not matter, but the base name must match a `[Movies]` value.

Active in YR: Yes. The same resolver is also used for scenario `[Basic] Intro`, `Brief`, `Win`, `Lose`, `Action`, `PostScore`, and `PreMapSelect`, but those consumers are outside this report's scope.

### 3.5 `[Movies]` source

`FUN_00674550 @ 0x00674550` reads `[Movies]` from the active art INI during `CDFileClass__Constructor @ 0x0052CD70`. It counts entries, reads each entry value into a 32-byte local buffer, checks for duplicates with `FUN_0048DF30`, duplicates the string with `0x007D5408`, and appends the pointer to `DAT_00ABF394`.

Active in YR: Yes. In repo data, `artmd.ini [Movies]` contains YR campaign cutscene names such as `A00_F00e`, `A01_F00e`, ... `S08_F01e`; base `art.ini` also contains older TS/RA2 entries, but YR startup uses the active art INI load path.

### 3.6 Mission-end consumer

`FUN_00685670 @ 0x00685670` is the scenario shutdown/end flow. The campaign final movie branch is:

```text
0x0068599E call FUN_0049F7A0
0x006859A3 if false -> exit display path
0x006859AB load ScenarioClass
0x006859B1 read Scenario+0x34AF (OneTimeOnly)
0x006859B9 if nonzero -> exit display path
0x006859BF read Scenario+0x34AC (EndOfGame)
0x006859C7 if zero -> map-select / next-scenario path
0x006859C9 read Scenario+0x34CC campaign index
0x006859D1 if -1 -> skip final movie but continue credits/score cleanup
0x006859D3 load DAT_00A83CFC campaign array
0x006859DE load CampaignClass* at index
0x006859E3 read Campaign+0x29C
0x006859E9 call FUN_005BF260(movie_index, 1, 1, 1)
0x006859EE call FUN_004C3E30
```

`FUN_005BF260 @ 0x005BF260` saves `DAT_00A8E378`, bounds-checks `0 <= movie_index < DAT_00ABF3A0`, clears `DAT_00A8E378`, calls `FUN_005BED40(1,1,1,0)`, then restores `DAT_00A8E378`. If the index is `-1` or out of range, it does nothing except restore the byte.

Active in YR: Conditional. The branch is in the live scenario shutdown path. It only attempts `FinalMovie` when all branch gates are met: game mode path reaches `FUN_00685670`, `FUN_0049F7A0()` returns true, `OneTimeOnly` is false, `EndOfGame` is true, and `Scenario+0x34CC != -1`.

### 3.7 File-name extension and BIK/VQA fallback

`FUN_005BED40 @ 0x005BED40` calls `FUN_005C0640` at `0x005BED54` before playback. Register evidence from assembly:

```text
0x005BED49 stores EDX argument
0x005BED4E push 1
0x005BED50 lea EDX, [ESP+0x54]     ; output filename buffer
0x005BED54 call 0x005C0640
```

`FUN_005C0640 @ 0x005C0640` receives the movie name in `ECX`, copies it to a local buffer, scans until NUL or `'.'`, and writes NUL at the first dot. It then calls `FUN_005FBF80(options, base_name)` for movie-progress bookkeeping, appends `.BIK` (`0x0082419C`) and checks file open through `CCFileClass` / virtual `+0x14`. If `.BIK` open fails, it appends `.VQA` (`0x008241A4`) to the same base and tries again. If either succeeds, it copies the resolved filename to the caller's output buffer and returns `1`; otherwise it returns `0`.

`FUN_005BED40` then checks the resolved filename's extension with `_strrchr('.', output)`. If the extension case-insensitively equals `.BIK` (`0x0082D9CC`), it uses the Bink path (`0x00432690`, `0x00432C70`, `0x00432700`). Otherwise it uses the legacy movie path opened by `0x005BFAA0` / `0x005BFF60`.

Active in YR: Yes for all movie-index playback calls, including `FinalMovie`. The BIK-before-VQA relationship is direct and load-bearing here.

## 4. INI Keys

| File / section | Key | Stock repo value | Binary reader | Effect | Active in YR |
|---|---|---|---|---|---|
| `ini/battlemd.ini [Battles]` | numbered campaign IDs | `1=ALL1`, `2=SOV1`, plus debug sections | `0x0046CE10` | creates/updates `CampaignClass` records | Yes |
| `ini/battlemd.ini [ALL1]`, `[SOV1]`, debug sections | `Scenario=` | mission map filename | `0x0046CCD0` | copied to `Campaign+0x9C`; later `ScenarioClass__Start_Scenario` uses it | Yes |
| same | `FinalMovie=` | all blank in repo copy | `0x0046CCD0` -> `0x004757D0` | blank leaves `Campaign+0x29C` unchanged, initialized `-1` | Conditional |
| same | `CD=` | `2` in YR repo copy | `0x0046CCD0` | copied to `Campaign+0x98`; used by load/CD UI paths | Yes |
| same | `Description=` | CSF label or debug raw text | `0x0046CCD0` | campaign selection text | Yes |
| same | `DebugOnly=` | set on debug sections | `0x0046CCD0` | switches description handling and debug suffix | Yes |
| `ini/artmd.ini [Movies]` | numbered movie values | `A00_F00e` .. `S08_F01e` | `0x00674550` | populates `DAT_00ABF394`; `FinalMovie=` must match one of these names | Yes |
| scenario map `[Basic]` | `EndOfGame=` | map-specific, default false | `0x0068A11F` | gates final campaign movie branch at `0x006859BF` | Conditional |
| scenario map `[Basic]` | `OneTimeOnly=` | map-specific, default false | `0x0068A1F3` | suppresses the final movie branch when true | Conditional |

## 5. Integration Points

| Stage | Function / address | Behavior | Active in YR |
|---|---|---|---|
| startup campaign data load | `0x0052CBA0` | loads `BATTLEMD*.INI`, with explicit `BATTLEMD.INI` fallback if needed | Yes |
| campaign record construction | `0x0046CE10` | iterates `[Battles]`, allocates `0x3A0` campaign objects, initializes `+0x29C=-1` | Yes |
| campaign record parse | `0x0046CCD0` | reads `FinalMovie` through movie-index resolver | Yes |
| movie table population | `0x00674550` from `0x0052CD70` | reads art `[Movies]` into global movie array | Yes |
| scenario load | `0x00689EA0`-range (`ScenarioClass::Read_INI_Basic`) | reads `[Basic] EndOfGame`, `OneTimeOnly`, and scenario movie keys | Yes |
| scenario end | `0x00685670` | if `EndOfGame` branch fires, reads `Campaign+0x29C` and calls `0x005BF260` | Conditional |
| movie-index playback | `0x005BF260`, `0x005BED40`, `0x005C0640` | bounds-checks index, resolves base name to `.BIK` then `.VQA`, plays selected backend | Conditional |

## 6. Current Rust Implementation Status

Rust currently implements Bink parsing/playback surfaces for main-menu/static tooling, but not campaign `FinalMovie` dispatch:

- `src/assets/asset_manager.rs` includes movie MIX names such as `movies01.mix` and `movies02.mix`.
- `src/assets/bink_file.rs`, `src/assets/bink_audio.rs`, and `src/render/bink_movie.rs` parse/decode/play BIK for current uses.
- `src/app_main_menu_shell_render.rs` hardwires main-menu `ra2ts_l.bik` / `ra2ts_s.bik`.
- No current `src/` hit for `FinalMovie`; no campaign `battlemd.ini` parser or `CampaignClass` equivalent was found in the slot scan.

Current Rust delta: missing for this target. The first Rust surface needed is data-model/parser support for campaign records and movie-index resolution against art `[Movies]`; playback can reuse/funnel into the broader movie player once it exists.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `BATTLEMD*.INI` loader | verified | `0x0052CBA0`, strings `0x00826198`, `0x008261A8` | none |
| `[Battles]` campaign object iteration | verified | `0x0046CE10`, string `0x0081B1D8` | none |
| `FinalMovie=` parser | verified | `0x0046CCD0`, string xref `0x0046CD1E -> 0x0081B1C8` | none |
| movie-name-to-index resolver | verified | `0x004757D0`, `0x0048DF30`, `0x007C8D20` | none |
| `[Movies]` source table | verified | `0x00674550`, caller `0x0052CD70`, repo `ini/artmd.ini` | exact merged loose/mod override order outside startup path deferred |
| mission-end `EndOfGame` final movie branch | verified | `0x006859BF..0x006859E9` | exact campaign progression before branch is outside scope |
| BIK-before-VQA filename fallback | verified | `0x005C0640`, strings `.BIK` at `0x0082419C`, `.VQA` at `0x008241A4` | exact legacy VQA renderer internals outside scope |
| stock `battlemd.ini` `FinalMovie` values | verified | `ini/battlemd.ini`, `ini/battle.ini` grep/read | retail MIX-embedded alternate/addon `BATTLEMD*.INI` not exhaustively searched |
| current Rust campaign dispatch | verified missing by scan | `rg FinalMovie src`; Bink files listed in prompt/context | future implementation design |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-01 - Which INI file family is active for YR campaign records? -> The active loader searches `BATTLEMD*.INI` and explicitly loads `BATTLEMD.INI` if the wildcard pass did not see it.` (evidence: `0x0052CBA0`, strings `0x00826198`, `0x008261A8`)
- `[RESOLVED] OQ-02 - Is base `battle.ini` the active YR source? -> Not in the inspected YR loader; it is comparison/base data, while this executable path uses `BATTLEMD*.INI`.` (evidence: `0x0052CBA0`)
- `[RESOLVED] OQ-03 - Where is `FinalMovie` read? -> `FUN_0046CCD0` reads it via `FUN_004757D0` and stores the returned movie index at `Campaign+0x29C`.` (evidence: `0x0046CD13..0x0046CD42`)
- `[RESOLVED] OQ-04 - What is the default for missing/blank `FinalMovie`? -> `Campaign+0x29C` is initialized to `-1`; blank/unknown values return the prior default.` (evidence: `0x0046CB7E`, `0x0046CEC2`, `0x004757D0`)
- `[RESOLVED] OQ-05 - Does `FinalMovie` store filename text or an index? -> It stores an integer index into `DAT_00ABF394`, not a filename string.` (evidence: `0x004757D0`, `0x0048DF30`, `0x006859E3`)
- `[RESOLVED] OQ-06 - Is matching case-sensitive? -> No; movie names compare through `FUN_007C8D20`, which performs case-insensitive comparison in the ordinary path.` (evidence: `0x0048DF30`, `0x007C8D20`)
- `[RESOLVED] OQ-07 - What table must `FinalMovie` names match? -> The `[Movies]` table populated by `FUN_00674550` from art INI into `DAT_00ABF394`.` (evidence: `0x00674550`, `ini/artmd.ini [Movies]`)
- `[RESOLVED] OQ-08 - Where is the campaign final movie consumed? -> `FUN_00685670` reads `Scenario+0x34CC`, loads the campaign pointer, reads `Campaign+0x29C`, and calls `FUN_005BF260`.` (evidence: `0x006859C9..0x006859E9`)
- `[RESOLVED] OQ-09 - What scenario condition gates final movie playback? -> `[Basic] EndOfGame` at `Scenario+0x34AC` must be true, and `OneTimeOnly` at `+0x34AF` must be false on this path.` (evidence: `0x0068A11F`, `0x0068A1F3`, `0x006859B1..0x006859C7`)
- `[RESOLVED] OQ-10 - Does an invalid movie index play anything? -> No; `FUN_005BF260` only calls playback if `0 <= index < DAT_00ABF3A0`.` (evidence: `0x005BF260`)
- `[RESOLVED] OQ-11 - Does the consumer append `.BIK` or expect the INI to include it? -> The playback filename helper strips any existing extension and appends `.BIK` first, then `.VQA` if BIK open fails.` (evidence: `0x005C067B..0x005C07BB`, strings `0x0082419C`, `0x008241A4`)
- `[RESOLVED] OQ-12 - Does the BIK/VQA relationship matter for this target? -> Yes, directly; `FinalMovie` feeds the same movie-index playback helper whose filename resolver tries BIK before VQA.` (evidence: `0x006859E9 -> 0x005BF260 -> 0x005BED40 -> 0x005C0640`)
- `[RESOLVED] OQ-13 - Do stock repo battle files request a final movie? -> No; all `FinalMovie=` lines in `ini/battlemd.ini` and `ini/battle.ini` are blank.` (evidence: repo INI scan)
- `[RESOLVED] OQ-14 - Does current Rust implement campaign `FinalMovie` dispatch? -> No direct `FinalMovie` surface was found; Rust currently has Bink parser/player and main-menu RA2TS usage only.` (evidence: `rg FinalMovie src`, prompt-listed Rust files)
- `[DEFERRED] OQ-15 - Which retail packed/addon `BATTLEMD*.INI` files outside repo data might set `FinalMovie`?` (category: `requires-different-system-context`; reason: this slot was scoped to repo `ini/battle.ini` and `ini/battlemd.ini` plus binary reader/consumer; next-step-if-pursued: enumerate loose/packed `BATTLEMD*.INI` from retail install and mods)
- `[DEFERRED] OQ-16 - Exact visible score/credits sequence after `0x006859EE -> FUN_004C3E30`?` (category: `out-of-scope`; reason: target stops at deciding what movie to play; next-step-if-pursued: investigate campaign end score/credits sequence)

## 9. Visual/UI Composition Ledger

This report does not claim pixel composition for the movie renderer. The only visual-facing claim is dispatch-level: `FinalMovie` reaches generic movie playback through `FUN_005BF260` if its gates pass.

| Order | Function / address | Condition / flag proof | Asset / frame | Rect / anchor | Palette / convert | Active for target? | Role |
|---|---|---|---|---|---|---|---|
| 1 | `0x00685670 -> 0x005BF260` | `EndOfGame=yes`, `OneTimeOnly=no`, campaign index valid, movie index in range | movie base from `DAT_00ABF394[Campaign+0x29C]` | movie renderer-owned | BIK or VQA backend-owned | Conditional | dispatch to final movie |

Asset role matrix:

| Asset | Loaded | Drawn | Visible in target | Content/preview | Chrome/container | Overlay | Transition-only | Inactive | Evidence |
|---|---|---|---|---|---|---|---|---|
| `*.BIK` from `FinalMovie` base | Conditional | Conditional | Conditional | yes | no | no | no | if `.BIK` file opens in `0x005C0640`, Bink path in `0x005BED40` |
| `*.VQA` from `FinalMovie` base | Conditional fallback | Conditional | Conditional | yes | no | no | no | if `.BIK` fails and `.VQA` opens in `0x005C0640` |

## 10. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| YR campaign records are loaded from `BATTLEMD*.INI`; each `FinalMovie=` resolves to a movie table index with blank/unknown preserving initialized `-1`. | `0x0052CBA0`, `0x0046CE10`, `0x0046CCD0`, `0x004757D0`; `ini/battlemd.ini` | missing | new campaign/battle parser surface; likely near map/campaign loading, plus art `[Movies]` data model | Parse `[Battles]` in source order, store `Scenario`, `CD`, `Description`, `DebugOnly`, and `FinalMovie` as optional resolved movie index/base name against `[Movies]`. | `campaign_final_movie_blank_resolves_to_none` | Do not store raw `FinalMovie` as a direct filename without validating against `[Movies]`; do not treat blank as empty filename to play. |
| End-of-mission final movie dispatch only runs through the `[Basic] EndOfGame=yes` branch, skips if `OneTimeOnly=yes`, and uses current campaign index `Scenario+0x34CC`. | `0x0068A11F`, `0x0068A1F3`, `0x006859B1..0x006859E9` | missing | future campaign scenario-end controller / game-screen state transition | On scenario shutdown, only attempt campaign `FinalMovie` when the scenario end state models gamemd's `EndOfGame` branch and campaign index is valid. | `campaign_endofgame_plays_final_movie_only_when_campaign_index_valid` | Do not play `FinalMovie` after every mission victory; it is not a generic per-mission `Win=` movie. |
| Movie filename resolution strips any existing extension and tries `.BIK` first, then `.VQA`; BIK extension chooses Bink path, non-BIK falls to legacy path. | `0x005C0640`, `0x005BED7B..0x005BEDA2`, strings `.BIK`/`.VQA` | partially present for main-menu BIK only | asset lookup/movie playback resolver; `src/assets/asset_manager.rs`, `src/render/bink_movie.rs`, future VQA fallback surface | Implement a generic movie resolver for movie-table base names with BIK-before-VQA fallback; route unsupported VQA to an explicit unsupported result until VQA exists. | `movie_resolver_strips_extension_and_prefers_bik_before_vqa` | Do not require `FinalMovie=A01_F00e.BIK`; gamemd strips extensions and resolves from base names. |

Stale Docs / Follow-up Docs:

- No exact prior `FINALMOVIE_CAMPAIGN_MOVIE_RESOLUTION` doc was found.
- If updating broader movie docs, use this wording: "`FinalMovie=` in `battlemd.ini` is parsed as a `[Movies]` index on `CampaignClass+0x29C`. It is consumed only by the scenario shutdown `EndOfGame` branch at `0x006859BF..0x006859E9`; stock repo battle files leave it blank, so this key does not drive a stock final movie unless addon/alternate battle INI data supplies one."

## Negative Facts / Do Not Do

- Do not parse `FinalMovie=` as an arbitrary filename. It must resolve through `[Movies]` or remain `-1`.
- Do not append `.bik` at campaign-parse time. gamemd stores an index and appends/tries extensions only at playback.
- Do not play `FinalMovie` for every mission win. The binary gates it behind scenario `[Basic] EndOfGame=yes`.
- Do not ignore VQA in the resolver contract. Even if VQA playback remains unsupported in Rust, the file lookup order is BIK first, then VQA.
- Do not treat base `battle.ini` as the active YR campaign loader path without additional evidence; this executable path uses `BATTLEMD*.INI`.

## Remaining Uncertainty

- Retail/addon `BATTLEMD*.INI` files outside the repo `ini/` copies were not exhaustively enumerated in this slot. The binary supports them through the wildcard loader.
- Exact score/credits behavior after `0x006859EE -> FUN_004C3E30` is outside this report.
- Exact VQA renderer behavior is outside this report; only the fallback decision is verified.

## Sources

- Ghidra decompile/disassembly: `0x0052CBA0`, `0x0046CE10`, `0x0046CCD0`, `0x004757D0`, `0x0048DF30`, `0x00674550`, `0x0068A000`/`ScenarioClass::Read_INI_Basic`, `0x00685670`, `0x005BF260`, `0x005BED40`, `0x005C0640`, `0x007C8D20`.
- String evidence: `FinalMovie` at `0x0081B1C8`, `Battles` at `0x0081B1D8`, `BATTLEMD.INI` at `0x00826198`, `BATTLEMD*.INI` at `0x008261A8`, `.BIK` at `0x0082419C`, `.VQA` at `0x008241A4`.
- INI evidence: `ini/battlemd.ini`, `ini/battle.ini`, `ini/artmd.ini [Movies]`, `ini/art.ini [Movies]`.
- Rust scan: `src/assets/asset_manager.rs`, `src/assets/bink_file.rs`, `src/assets/bink_audio.rs`, `src/render/bink_movie.rs`, `src/app_main_menu_shell_render.rs`; no `FinalMovie` hit in `src/`.
