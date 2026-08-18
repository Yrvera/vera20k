# Skirmish Create Random Map 0x583 Broad Recheck - Ghidra Research Report

**Address(es):** `0x005E69D3`, `0x005E6A11`, `0x005E8590`, `0x00597730`, `0x00641DB0`, `0x005E7160`, `0x005E7BF0`, `0x00684620`, `0x00597A10`, `0x00598960`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** Choose Map dialog `0x6B` command `0x583` through random-map setup acceptance, `RandMap.Sed`/`RandMap.img` side effects, sentinel selected-record update, modal return behavior, and launch-time `.SED` reload/generation handoff.  
**Non-Scope:** random terrain/noise formulas inside `0x00598960`, random-map dialog visual layout, exact generated per-seed RGB terrain formulas, broad MPModes/session packing, and generic Choose Map listbox/button paint outside the `0x583` command boundary.  
**Confidence:** High for UI-shell command flow and Rust-facing handoff; Medium only for runtime UX of malformed external `.SED` or missing/corrupt `RandMap.img`, which was statically bounded but not runtime-captured.  
**Active in YR:** Conditional. Active in standard YR offline Skirmish when the player clicks Create Random Map in Choose Map `0x6B` and the random-map dialog returns accepted result `1`; launch branch is active when selected filename has `.SED` suffix, normally `RandMap.Sed`.

## 0. Working Notes Gate

**Target question:** What exact Rust-facing command flow must replace the current log-only Create Random Map `0x583` path, from Choose Map modal through sentinel commit and launch reload?

**Non-goals:** Do not rediscover generic RMG terrain formulas, full generated-map placement internals, random-map dialog visual controls, or ordinary stock-map selection behavior except where those boundaries prove the `0x583` handoff.

**Evidence needed to mark COMPLETE:** Decompile plus assembly for the `0x583 -> 0x005E8590` branch, decompile plus caller/xref evidence for `.SED` writer/reader and `RandMap.img` loader, decompile plus caller evidence for selected-map token writes and launch `.SED` reload/generation, INI/default source for mode gating, and current Rust scan for app/state/preview/launch deltas.

**Stop conditions:** Stop after the UI-shell handoff is implementable and non-noop acceptance tests are defined; defer terrain formulas, native screenshot RGB, malformed-file runtime UX, and random-map dialog visuals.

Prior state row: **Partial/high-confidence reports exist; proceed to gaps + verification only.** This report reconciles the prior RMG reports listed in Sources and spot-checks only the handoff-critical boundaries.

## 1. Overview

Create Random Map `0x583` is a real modal command, not an ordinary stock-map row and not a log-only action. The callback hides/suspends the chooser, calls `0x005E8590`, and only continues when the random-map dialog returns exactly `1`. On success, native saves the current `MapSeedClass` state to `RandMap.Sed`, replaces the preview wrapper from runtime `RandMap.img`, update-or-appends one official `RandMap.Sed` scenario record, reselects/loads that record through normal list/selected-map paths, and closes through the ordinary Use Map accept helper.

Launch later still sees the selected filename `RandMap.Sed`. `ScenarioClass__Read_Scenario @ 0x00684620` detects the `.SED` suffix, loads `[RandomMap]` seed/options through `0x00597A10`, calls `0x00598960(0,0)` only on seed-load success, and leaves the scenario filename as the original `.SED` token while generated map state lives in memory.

## 2. Key Offsets / State

| Item | Meaning | Evidence | Active in YR |
|---|---|---|---|
| command `0x583` | Choose Map Create Random Map button command | callback assembly `0x005E69D3 SUB EAX,0x583`, `0x005E69D8 JZ 0x005E69FD` | Conditional: player click |
| `0x005E8590` | accepted random-map setup path | call at `0x005E6A11`; decompile `0x005E8590` | Conditional: dialog result `1` |
| `DAT_00ABDFD8` | global `MapSeedClass` seed/options object | save call `ECX=0x00ABDFD8` at `0x005E85D6`; load/generate at `0x0068496F/0x00684984` | Conditional |
| `RandMap.Sed` | selected scenario filename and seed/options file | string pushed at `0x005E85D1`, constructor at `0x005E8683`, selected filename copy in `0x005E7BF0` | Conditional |
| `RandMap.img` | runtime generated preview image | loader call `0x005E8626 -> 0x00641DB0`; xrefs from setup/return/init | Conditional |
| scenario record `+0x58` | filename token copied to `DAT_00A8B8E0` and `ScenarioClass+0x125C` | `0x005E7BF0` decompile | Yes / Conditional for sentinel |
| scenario record `+0x15C` | digest/source text updated from `0x005E84D0` | `0x005E8590` decompile and `0x005E8656..0x005E8683` | Conditional |
| scenario record `+0x17C` | official flag; new sentinel passes `1` | constructor args `0x005E8674..0x005E8683` | Conditional |
| scenario record min/max | new sentinel hardcoded min `2`, max `4` | `0x005E866E PUSH 4`, `0x005E8670 PUSH 2` | Conditional |
| `ScenarioClass+0x34BD` | random flag set by `.SED` suffix detection | `0x00684620` decompile; read at `0x00684961` | Conditional |
| `ScenarioClass+0x125C` | retained selected scenario filename; remains `RandMap.Sed` after generation | `0x005E7BF0`, `0x00684995..0x006849BF` | Conditional |

## 3. Core Logic

### 3.1 The `0x583` command enters a separate setup branch

Verified behavior: The Choose Map callback compares command ids, identifies `0x583`, hides/suspends the modal, then calls `0x005E8590`. If `0x005E8590` returns `-1`, it skips sentinel/list/accept work and returns through cleanup/show behavior.

Evidence: assembly context `0x005E69D3..0x005E6A1F`: `SUB EAX,0x583`, `JZ 0x005E69FD`, `CALL 0x005E8590`, `CMP EBX,-1`, `JZ 0x005E6B47`. Active in YR: Conditional, standard dialog command path.

Implementation implication: The Rust app branch at [app.rs](C:/Users/enok/Documents/ra2-rust-game/src/app.rs:717) cannot remain an invisible log. A player click must either enter the verified random-map setup flow or present an explicit blocked UI state until that setup exists.

### 3.2 `0x005E8590` only commits after accepted dialog result `1`

Verified behavior: `0x005E8590` calls the random-map dialog wrapper `0x00595BC0`; any return other than `1` returns `-1`. Accepted result writes byte `DAT_008316D4 = 1`, then saves `DAT_00ABDFD8` to `RandMap.Sed`.

Evidence: decompile `0x005E8590`; assembly `0x005E85C1 CALL 0x00595BC0`, `0x005E85C6 CMP EAX,0x1`, `0x005E85CB OR EAX,-1`, then accepted path `0x005E85D1 PUSH RandMap.Sed`, `0x005E85D6 MOV ECX,0x00ABDFD8`, `0x005E85DB MOV byte ptr [0x008316D4],1`, `0x005E85E2 CALL 0x00597730`. Active in YR: Conditional on dialog accept.

Tiny detail: The cancel path is not a partial sentinel path. It returns `-1` before saving `.SED`, replacing preview, or scanning records.

### 3.3 `RandMap.Sed` is persisted seed/options, not a normal map

Verified behavior: `0x00597730` is a wrapper. With non-null filename it dispatches `MapSeedClass` vtable `+0x8`; prior writer-layout report resolves this to writer `0x00597760`, which emits `[RandomMap]` with `Description` plus sixteen decimal integer keys. The launch reader wrapper `0x00597A10` dispatches vtable `+0x4`; prior report resolves this to reader `0x00597A30`.

Evidence: decompile `0x00597730` and assembly `0x00597730..0x00597740`; decompile `0x00597A10` and assembly `0x00597A10..0x00597A1E`; writer-layout report vtable evidence. Active in YR: Conditional on accepted setup and `.SED` launch.

Material layout from prior verified report: section `RandomMap`; keys `Description`, `Width`, `Height`, `NumPlayers`, `Seed`, `MapType`, `Theater`, `Time`, `RegionSize`, `Ruggedness`, `Accessibility`, `WaterAmount`, `Tiberium`, `TiberiumLayout`, `Vegetation`, `UrbanPresence`, `Resources`. `Description` is comma-separated hex UTF-16 code units; integer values are signed decimal.

### 3.4 `RandMap.img` replaces preview source and has null-inner fallback semantics

Verified behavior: Accepted setup destroys any old `DAT_00AC1154` preview wrapper, allocates a fresh wrapper, stores it, and calls `0x00641DB0("RandMap.img")`. The loader destroys old inner surface, loads a PCX-style image into a temp `BSurface`, requires nonzero width/height, constructs a destination `DSurface` of decoded dimensions, copies the image, and returns `1`; failure can leave a wrapper with null inner surface.

Evidence: setup assembly `0x005E85E7..0x005E8626`; decompile `0x00641DB0`; assembly `0x00641DD8..0x00641DE4` old-inner destroy, `0x00641E24..0x00641E57` destination allocate/construct, `0x00641EAB..0x00641EDC` failure cleanup. Active in YR: Conditional.

Tiny detail: `0x005E8590` itself does not inspect loader success before updating records; fallback checks occur in surrounding setup/return/init branches when wrapper `+0` is null. Rust must not treat "random sentinel exists" as "drawable random preview exists."

### 3.5 One sentinel record is updated in place or appended with native fields

Verified behavior: After preview replacement, `0x005E8590` scans the scenario-record vector and calls the filename comparer (`record+0x58 == RandMap.Sed`). If found, it updates display/name from `DAT_00ABE050` and digest/source from `0x005E84D0`. If not found, it allocates `0x1BC` bytes and constructs one official record with file `RandMap.Sed`, name `DAT_00ABE050`, digest `0x005E84D0`, no GameModes list, min players `2`, and max players `4`.

Evidence: decompile `0x005E8590`; xref `0x005E8590` only from callback `0x005E6A11`; assembly `0x005E8636..0x005E871F`; constructor argument context `0x005E866E PUSH 4`, `0x005E8670 PUSH 2`, `0x005E8674 PUSH 1`, `0x005E8683 CALL 0x0069A980`. Active in YR: Conditional.

Current Rust correction: [skirmish_scenarios.rs](C:/Users/enok/Documents/ra2-rust-game/src/skirmish_scenarios.rs:107) now gives the sentinel `file_name = RandMap.Sed`, min `2`, max `4`, and `official = true`. That part no longer matches older reports' stale Rust delta. Missing pieces remain seed/options persistence, digest/source, preview image, app-level command routing, and launch `.SED` generation.

### 3.6 Accepted `0x583` re-enters ordinary selection/accept semantics

Verified behavior: The command branch uses listbox and selected-record helpers after `0x005E8590` returns an index. It rebuilds/reselects through normal map-list state, calls selected-record load helper `0x005E7BF0`, checks/falls back preview if needed, and then calls `0x005E7160`, the ordinary Use Map accept helper.

Evidence: callback assembly `0x005E6A25..0x005E6B41`, including `0x005E6B28 CALL 0x005E7BF0` and `0x005E6B2F CALL 0x005E7160`; decompile `0x005E7160` reads `LB_GETCURSEL 0x188` / `LB_GETITEMDATA 0x199` from map list `0x553`, resolves the record index, reads mode list `0x6EB`, writes selected globals, and closes with result `1`. Active in YR: Conditional.

Tiny detail: Native does not encode random map as `None`, a negative index, or a side-channel outside the scenario record vector. It commits an ordinary record index whose filename is `RandMap.Sed`.

### 3.7 Selected-map token load copies `RandMap.Sed` to launch identity

Verified behavior: `0x005E7BF0(index)` copies record `+0x58` into `DAT_00A8B8E0` and then into `ScenarioClass+0x125C`; for the random sentinel that token is `RandMap.Sed`. It also copies record `+0x15C` digest to `DAT_00A8BAE2` and record `+0x17C` official flag to `DAT_00A8BB08`.

Evidence: decompile `0x005E7BF0`; selected-record helper is called from `0x005E6B28` and parent return paths. Active in YR: Yes for selected records, Conditional for sentinel.

Implementation implication: Keep a real selected map token/file string for random maps. Do not collapse random map to a boolean "auto" or a display-only sentinel.

### 3.8 Launch reload/generation is `.SED` suffix based and retains filename

Verified behavior: `ScenarioClass__Read_Scenario @ 0x00684620` copies the selected filename locally, compares the final suffix with `.SED`, writes `ScenarioClass+0x34BD`, and branches away from normal INI map loading when random. On the random branch, it calls `0x00597A10(local_filename)` with `ECX=0x00ABDFD8`; only if that returns true does it call `0x00598960(0,0)` and `ScenarioClass__Post_Map_Init(1)`. After either random success or failure it copies the original local filename back into `ScenarioClass+0x125C`.

Evidence: decompile `0x00684620`; assembly `0x00684961..0x00684990` for flag check, seed-load, generation call; `0x00684995..0x006849BF` for filename copy-back. Active in YR: Conditional on `.SED` suffix.

Tiny details:

- Predicate is suffix `.SED`, not a full string special-case for `RandMap.Sed`.
- Launch passes `0,0` to `0x00598960`; preview repaint paths are not used.
- The playable map state is generated in memory; no generated `.map` filename replaces `RandMap.Sed`.

## 4. INI / Data Keys

| Source | Key / field | Native value / behavior | Evidence | Active in YR |
|---|---|---|---|---|
| `ini/mpmodesmd.ini:[Battle] 1` | fifth field | `true`; Battle admits random-map sentinel | local INI line `1=GUI:Battle,...,true`; prior mode parser reports | Yes |
| `ini/mpmodesmd.ini:[FreeForAll] 2` | fifth field | `true`; Free For All admits random-map sentinel | local INI line `2=GUI:FreeForAll,...,true` | Conditional by selected mode |
| `ini/mpmodesmd.ini` rows `3..9` | fifth field | `false` for Team Game/Megawealth/Duel/MeatGrind/NavalWar/UnholyAlliance/Cooperative | local INI scan | Conditional by selected mode |
| `RandMap.Sed:[RandomMap]` | seed/options keys | reader consumes layout listed in Section 3.3 | writer/reader reports, `0x00597730`, `0x00597A10` | Conditional |

No `rules.ini` / `rulesmd.ini` key directly gates command `0x583`; generator support reads broader RMG defaults later and is outside this shell-command slice.

## 5. Integration Points

| Integration | Behavior | Evidence | Active in YR |
|---|---|---|---|
| Choose Map `0x6B` callback | `0x583` calls `0x005E8590`; `-1` skips accepted side effects | assembly `0x005E69D3..0x005E6A1F` | Conditional |
| Random-map dialog accept | only return `1` enters save/preview/sentinel path | `0x005E8590` decompile; `0x005E85C1..0x005E85CE` | Conditional |
| `.SED` write | `DAT_00ABDFD8` saved to `RandMap.Sed` before preview and record updates | `0x005E85D1..0x005E85E2`, `0x00597730` | Conditional |
| Preview | setup loads `RandMap.img` into `DAT_00AC1154`, with later null-inner fallback | `0x005E861A..0x005E8626`, `0x00641DB0`, xrefs to return/init callers | Conditional |
| Sentinel record | update existing by filename or append one official min2/max4 record | `0x005E8636..0x005E871F` | Conditional |
| Modal commit | ordinary selected-map accept helper `0x005E7160` closes result `1` | `0x005E6B2F`, `0x005E7160` | Conditional |
| Launch | `.SED` suffix loads seed and generates map in memory | `0x00684961..0x00684990`, `0x00597A10`, `0x00598960` | Conditional |

## 6. Current Rust Implementation Status

| Rust surface | Status vs binary | Evidence |
|---|---|---|
| Button recognition | present but log-only | [app.rs](C:/Users/enok/Documents/ra2-rust-game/src/app.rs:717) logs that random map generation is not implemented |
| Modal helper for sentinel | present but unused by app `0x583` branch | [state.rs](C:/Users/enok/Documents/ra2-rust-game/src/ui/skirmish_shell/state.rs:201) `ChooseMapModalState::create_random_map` upserts and refreshes rows |
| Native sentinel identity/capacity | mostly present | [skirmish_scenarios.rs](C:/Users/enok/Documents/ra2-rust-game/src/skirmish_scenarios.rs:107) creates `RandMap.Sed`, min `2`, max `4`, official `true` |
| Sentinel upsert | present | [skirmish_scenarios.rs](C:/Users/enok/Documents/ra2-rust-game/src/skirmish_scenarios.rs:210) updates one sentinel by kind |
| Random seed/options object | missing | no Rust `[RandomMap]` seed/options model found by Codegraph/`rg` |
| `RandMap.Sed` writer/reader | missing | no `.sed` branch in [app_init.rs](C:/Users/enok/Documents/ra2-rust-game/src/app_init.rs:257) or [app_list_maps.rs](C:/Users/enok/Documents/ra2-rust-game/src/app_list_maps.rs:385) |
| `RandMap.img` preview source | missing | no `RandMap.img` branch found; preview rendering only names GUI label at [app_skirmish_shell_render.rs](C:/Users/enok/Documents/ra2-rust-game/src/app_skirmish_shell_render.rs:1922) |
| Launch-time generation | missing | requested map strings route through normal map loader at [app_init.rs](C:/Users/enok/Documents/ra2-rust-game/src/app_init.rs:257) |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| Working notes gate | verified | Section 0 | none |
| `0x583` callback command boundary | verified | assembly `0x005E69D3..0x005E6A1F` | no full callback function boundary needed for this slice |
| `0x005E8590` setup | verified | decompile plus assembly `0x005E85C1..0x005E871F` | random-map dialog visual internals out-of-scope |
| `.SED` save wrapper `0x00597730` | verified | decompile plus assembly `0x00597730..0x00597740`; writer-layout report | concrete writer already covered by sibling |
| `RandMap.img` loader `0x00641DB0` | verified | decompile plus assembly `0x00641DD8..0x00641E57`; loader/dimensions reports | runtime corrupt-image screenshot deferred |
| Sentinel update/append | verified | `0x005E8636..0x005E871F`, constructor args `0x005E866E..0x005E8683` | none |
| Accept helper `0x005E7160` | verified | decompile plus caller `0x005E6B2F` | none for command handoff |
| Selected-token loader `0x005E7BF0` | verified | decompile; prior reports | none for filename identity |
| Launch `.SED` branch `0x00684620` | verified | decompile plus assembly `0x00684961..0x006849BF` | malformed external `.SED` UX deferred |
| Generator `0x00598960` launch boundary | verified for handoff | call `0x00684989`, generator report | formulas out-of-scope |
| MPModes random flag defaults | verified | `ini/mpmodesmd.ini` local scan plus prior mode reports | none for standard offline modes |
| Current Rust surfaces | verified | Codegraph context plus `rg`/file reads | implementation not performed |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-01 - Is command 0x583 active in standard YR Choose Map? -> Yes, conditionally on player click; callback subtracts 0x583 and jumps into setup branch.` (evidence: `0x005E69D3..0x005E69FD`)
- `[RESOLVED] OQ-02 - What commits or aborts the branch? -> `0x005E8590` returns `-1` unless `0x00595BC0` returns exactly `1`.` (evidence: `0x005E85C1..0x005E85CE`)
- `[RESOLVED] OQ-03 - Is `RandMap.Sed` saved before sentinel record update? -> Yes, `DAT_00ABDFD8` saves to `RandMap.Sed` before preview replacement and record scan.` (evidence: `0x005E85D1..0x005E8626`)
- `[RESOLVED] OQ-04 - Does setup use `RandMap.img` or PreviewPack? -> It loads `RandMap.img` through `0x00641DB0`; normal PreviewPack is not the random preview source.` (evidence: `0x005E861A..0x005E8626`, `0x00641DB0`)
- `[RESOLVED] OQ-05 - Can loader success be assumed from wrapper allocation? -> No; failure can leave wrapper `+0` null and callers/fallbacks must treat drawable preview separately.` (evidence: `0x00641EAB..0x00641EDC`, loader report)
- `[RESOLVED] OQ-06 - Does native create duplicate random sentinel records? -> No; it scans existing records for filename `RandMap.Sed` and updates in place before append fallback.` (evidence: `0x005E8636..0x005E871F`)
- `[RESOLVED] OQ-07 - What native fields are on a newly appended sentinel? -> file `RandMap.Sed`, display `DAT_00ABE050`, digest `0x005E84D0`, official `1`, min `2`, max `4`.` (evidence: `0x005E866E..0x005E8683`)
- `[RESOLVED] OQ-08 - Is the sentinel committed through an ordinary selected-record path? -> Yes, branch calls `0x005E7BF0` and `0x005E7160`; accept helper reads listbox item data and writes selected globals.` (evidence: `0x005E6B28..0x005E6B2F`, `0x005E7160`)
- `[RESOLVED] OQ-09 - What token reaches launch? -> The record filename `RandMap.Sed` is copied to `DAT_00A8B8E0` and `ScenarioClass+0x125C`.` (evidence: `0x005E7BF0`)
- `[RESOLVED] OQ-10 - What selects launch-time random generation? -> suffix compare against `.SED`, not a special negative index or full `RandMap.Sed` compare.` (evidence: `0x00684620`, `0x00684961`)
- `[RESOLVED] OQ-11 - Does launch generate only after seed load succeeds? -> Yes; `0x00597A10` false return skips `0x00598960`.` (evidence: `0x00684975..0x00684989`)
- `[RESOLVED] OQ-12 - Does launch replace filename with generated map path? -> No; it copies the original local filename back into `ScenarioClass+0x125C`.` (evidence: `0x00684995..0x006849BF`)
- `[RESOLVED] OQ-13 - Which standard modes admit the random sentinel? -> Battle and Free For All have fifth field `true`; other local YR modes scanned have `false`.` (evidence: `ini/mpmodesmd.ini:8`, `ini/mpmodesmd.ini:20`, local scan)
- `[RESOLVED] OQ-14 - Is current Rust still missing the app command? -> Yes; the app `CreateRandomMap0x583` branch logs only.` (evidence: [app.rs](C:/Users/enok/Documents/ra2-rust-game/src/app.rs:717))
- `[RESOLVED] OQ-15 - Is current Rust sentinel still missing official/min/max? -> No; current sentinel has `official=true`, min `2`, max `4`.` (evidence: [skirmish_scenarios.rs](C:/Users/enok/Documents/ra2-rust-game/src/skirmish_scenarios.rs:107))
- `[DEFERRED] OQ-16 - Exact random terrain/noise formulas inside `0x00598960`.` (category: out-of-scope; reason: generic RMG internals are not needed for this command-flow handoff; next-step-if-pursued: use dedicated generator formula reports)
- `[DEFERRED] OQ-17 - Runtime visual UX for corrupt/missing `RandMap.img`.` (category: needs-runtime-debugger; reason: static null-inner/fallback branches are known, screenshot output was not captured; next-step-if-pursued: native runtime file-corruption experiment)
- `[DEFERRED] OQ-18 - Malformed external `.SED` player-facing failure UX.` (category: needs-runtime-debugger; reason: static load-failure gate is known, exact modal/log output needs runtime experiment; next-step-if-pursued: launch crafted malformed `.SED`)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| `0x583` is an accepted-dialog setup command; cancel/no-accept returns `-1` and must not alter selected map or sentinel. | `0x005E69D3..0x005E6A1F`, `0x005E85C1..0x005E85CE` | missing: app logs only | [app.rs](C:/Users/enok/Documents/ra2-rust-game/src/app.rs:717), modal action state | Route button into a real random-map setup state; only accepted setup may update sentinel/selection. | Click Create Random Map then cancel setup: previous map token and preview remain unchanged. | Do not upsert/commit `RandMap.Sed` on mere button click. |
| Accepted setup saves `MapSeedClass` state to native `RandMap.Sed` before preview and record commit. | `0x005E85D1..0x005E85E2`, `0x00597730`, writer-layout report | missing seed/options model and writer | new random-map seed/options model; app/modal state; launch state | Store/serialize `[RandomMap]` seed/options with native key names/value encoding before committing sentinel. | Accepted Create Random Map produces a deterministic seed/options object with native defaults/clamps and selected token `RandMap.Sed`. | Do not model random map as display-name-only sentinel state. |
| Accepted setup replaces preview from runtime `RandMap.img`; drawable image dimensions/content come from generated preview, not stock PreviewPack. | `0x005E861A..0x005E8626`, `0x00641DB0`, dimensions report | missing | preview cache/render path; image decoder surface | Add random-sentinel preview source for `RandMap.img` or equivalent generated preview image, with null/failed image not reusing stale old preview. | After accepting random setup, chooser/setup preview uses random image source; corrupt/missing image does not leave previous concrete-map thumbnail. | Do not decode `RandMap.Sed` as `[PreviewPack]`; do not hardcode thumbnail dimensions. |
| One `RandMap.Sed` record is update-or-append; new record is official and min/max `2/4`. | `0x005E8636..0x005E871F`, `0x005E866E..0x005E8683` | mostly present for identity/capacity; missing digest/source and app command use | [skirmish_scenarios.rs](C:/Users/enok/Documents/ra2-rust-game/src/skirmish_scenarios.rs:107), [state.rs](C:/Users/enok/Documents/ra2-rust-game/src/ui/skirmish_shell/state.rs:201) | Reuse existing sentinel identity/capacity, add setup digest/source if needed, and call the helper only from accepted setup. | Create Random Map twice leaves one sentinel whose display/setup metadata updates, not two rows. | Do not append duplicate random sentinel rows or add it as a permanent loose-map scan result. |
| Successful `0x583` reselects/loads the sentinel and closes through ordinary Use Map accept semantics. | `0x005E6B28..0x005E6B2F`, `0x005E7160`, `0x005E7BF0` | partial modal accept exists, but random command does not commit | modal state, app commit path, preview invalidation/session token | Treat successful random setup as selecting and accepting the `RandMap.Sed` record through normal committed-selection state. | Accepted random setup closes chooser and Start uses `selected_map_file = "RandMap.Sed"`. | Do not encode random map as `None`, `auto`, or a negative index. |
| Launch detects `.SED`, loads `[RandomMap]`, calls `0x00598960(0,0)` on success, and retains `RandMap.Sed` filename. | `0x00684961..0x006849BF`, `0x00597A10`, generator report | missing: normal map loader handles all requested map names | [app_init.rs](C:/Users/enok/Documents/ra2-rust-game/src/app_init.rs:257), [app_list_maps.rs](C:/Users/enok/Documents/ra2-rust-game/src/app_list_maps.rs:385), future generator | Add `.sed` random-map launch branch before normal concrete-map lookup; generated map state should be in memory while identity remains `.SED`. | Start after accepting random map does not report "Map 'RandMap.Sed' not found"; it reaches random generation or an explicit not-implemented launch error. | Do not require `RandMap.Map`, a generated loose `.map`, or preview image bytes as gameplay terrain. |
| Launch generation is separate from preview generation; it passes `(0,0)` and does not repaint modal preview. | `0x00684980..0x00684990`; generator report | missing | random map generator entry, render preview cache | Keep gameplay generation independent from UI preview textures/window handles. | Same seed/options can launch without any existing preview texture. | Do not reuse `RandMap.img` as authoritative map data. |

## 10. Negative Facts / Do Not Do

- Do not leave Create Random Map as silent log-only player behavior. Active in YR: No; `0x583` calls `0x005E8590` on a live dialog branch. Evidence: `0x005E69D3..0x005E6A11`.
- Do not create or commit `RandMap.Sed` when the random-map setup dialog is canceled. Active in YR: No; `0x005E8590` returns `-1` unless result is `1`. Evidence: `0x005E85C1..0x005E85CE`.
- Do not append duplicate random-map records. Active in YR: No; existing records are scanned and updated first. Evidence: `0x005E8636..0x005E871F`.
- Do not treat `RandMap.Sed` as a normal playable map INI. Active in YR: No; launch `.SED` branch calls seed reader and generator instead of normal `ScenarioClass__Read_Scenario_INI`. Evidence: `0x00684961..0x006849C9`.
- Do not replace the selected filename with `RandMap.Map` or a generated loose filename. Active in YR: No; `ScenarioClass+0x125C` is restored from the original local `.SED` filename. Evidence: `0x00684995..0x006849BF`.
- Do not use `RandMap.img` as terrain or launch data. Active in YR: No; it is a UI preview image channel. Evidence: loader `0x00641DB0`; launch calls seed/generator `0x00597A10 -> 0x00598960`.
- Do not draw a random preview by decoding `[PreviewPack]` from `RandMap.Sed`. Active in YR: No; setup loads `RandMap.img`. Evidence: `0x005E861A..0x005E8626`.
- Do not make random-map row highlight alone commit selection or refresh parent preview. Active in YR: No for ordinary browsing; commit is through accept/return paths, with `0x583` as a command-side exception. Evidence: `0x005E7160`, prior Choose Map preview-refresh report.
- Do not put random generation in deterministic `sim/` tick logic. Active in YR: No; generation runs during scenario/map load before playable state. Evidence: `0x00684620 -> 0x00598960 -> ScenarioClass__Post_Map_Init`.

## 11. Stale Docs / Follow-up Docs

- `SKIRMISH_CREATE_RANDOM_MAP_0X583_SETUP_PATH_GHIDRA_REPORT.md` Section 6 Rust status is stale for sentinel min/max/official. Replacement wording: "Current Rust now models the random sentinel identity with `RandMap.Sed`, min players `2`, max players `4`, and `official=true` in `src/skirmish_scenarios.rs`; remaining deltas are app-level `0x583` routing, seed/options `.SED` state, `RandMap.img` preview, digest/source metadata, and launch `.SED` generation."
- `SKIRMISH_RANDOM_MAP_BRANCH_AFTER_SELECTED_MAP_LOAD_GHIDRA_REPORT.md` Rust status remains current for launch: `RandMap.Sed` still routes to normal map lookup unless a future `.sed` branch is implemented.
- `SKIRMISH_RANDMAP_IMG_PREVIEW_LOADER_00641DB0_GHIDRA_REPORT.md` remains current for preview source and null-inner warning; no contradiction found.

## Sources

- Fresh read-only Ghidra decompile: `0x005E8590`, `0x006ACEE0`, `0x005E7160`, `0x005E7BF0`, `0x00684620`, `0x00641DB0`, `0x00597730`, `0x00597A10`.
- Fresh read-only Ghidra assembly/xrefs: `0x005E69D3..0x005E6B41`, `0x005E85C1..0x005E871F`, `0x00684961..0x006849BF`, `0x00641DD8..0x00641E57`; xrefs to `0x005E8590`, `0x00641DB0`, `0x00597730`, `0x00597A10`, `0x00598960`, `0x005E7160`.
- Prior reports reconciled: `SKIRMISH_CREATE_RANDOM_MAP_0X583_SETUP_PATH_GHIDRA_REPORT.md`, `SKIRMISH_RANDOM_MAP_BRANCH_AFTER_SELECTED_MAP_LOAD_GHIDRA_REPORT.md`, `SKIRMISH_RANDOM_MAP_GENERATOR_00598960_GHIDRA_REPORT.md`, `SKIRMISH_RANDMAP_SED_WRITER_00597730_LAYOUT_GHIDRA_REPORT.md`, `SKIRMISH_RANDMAP_IMG_PREVIEW_LOADER_00641DB0_GHIDRA_REPORT.md`, `GENERATETERRAINPREVIEW_RANDMAP_DIMENSIONS_COLORS_GHIDRA_REPORT.md`, `SKIRMISH_CHOOSE_MAP_0X6B_CURRENT_MODAL_RECHECK_GHIDRA_REPORT.md`.
- INI checked: `C:/Users/enok/Documents/ra2-rust-game/ini/mpmodesmd.ini`.
- Current Rust scan: Codegraph context plus `src/app.rs`, `src/ui/skirmish_shell/state.rs`, `src/skirmish_scenarios.rs`, `src/app_init.rs`, `src/app_list_maps.rs`, `src/app_skirmish_shell_render.rs`.
