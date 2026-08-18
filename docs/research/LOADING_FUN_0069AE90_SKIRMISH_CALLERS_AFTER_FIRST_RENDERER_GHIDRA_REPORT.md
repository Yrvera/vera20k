# Loading FUN_0069AE90 Skirmish Callers After First Renderer - Ghidra Report

**Address(es):** `FUN_0069AE90 @ 0x0069AE90`; post-first-render anchor `ScenarioClass__Full_Init @ 0x00687588..0x00687594`.
**Investigation Mode:** exhaustive-slice.
**Claimed Scope:** every verified `FUN_0069AE90` caller that can run on the standard offline Skirmish load path after the first verified renderer `0x00552D60`, including nested callers reached from `ScenarioClass__Full_Init`.
**Non-Scope:** progress draw geometry, `mmpb.shp` marker semantics, LS background composition, campaign-only loading text, and random-map terrain-generation progress before the first verified renderer.
**Confidence:** High for normal selected-map standard offline Skirmish milestone caller order; Medium for random-map pre-Full_Init generator exclusion because this slot did not drain random-map generation ordering beyond proving it is not the normal selected-map after-first-render sequence.
**Active in YR:** Yes for standard offline Skirmish (`g_GameMode == 5`) through `ScenarioClass__Read_Scenario -> ScenarioClass__Full_Init`.

## Working Notes

Target question: enumerate every standard offline Skirmish load-path caller of `FUN_0069AE90` after `0x00552D60`, with argument values, order, gates, and Rust implications.
Non-goals: do not re-decode first renderer visuals, progress geometry, campaign loading text, or write Rust.
Evidence needed to mark COMPLETE: direct caller/xref list for `0x0069AE90`; decompile plus assembly context for each after-first-render caller; standard Skirmish liveness gates; ledger rows with visible/suppressed implication.
Stop conditions: stop when post-`0x00552D60` `Full_Init` sequence, nested `Init_Theater`, nested `Read_Map_Section_And_IsoMapPacks`, nested `Read_INI_Basic`, nested `Post_Map_Init`, and post-`Full_Init` finalization are classified; exclude xrefs not on the target path with evidence.

## 1. Overview

The standard offline Skirmish loading progress path is milestone-driven, not smooth. After `ScenarioClass__Full_Init` calls the first verified renderer `0x00552D60`, it immediately calls `FUN_0069AE90(3)`, then continues through nested theater, INI, map-pack, object, post-map, and finalization milestones.

The load-bearing finding for Rust is that not every caller means a visible repaint: `FUN_0069AE90` first halves random-map inputs, then compares requested percent against current percent and only calls `FUN_00643C50` when the requested value is strictly greater than current. This makes lower or duplicate milestones, such as `6` after `8`, the outer `58` after inner `60`, and later duplicate `60`, visible no-ops.

## 2. Key Fields And Gates

| Field / global | Meaning in this slice | Evidence | Active in YR? |
|---|---|---|---|
| `g_GameMode == 5` | standard offline Skirmish path; non-campaign progress asset and non-network-wait behavior | `ScenarioClass__Read_Scenario @ 0x00684620`; `FUN_00684370 @ 0x00684370` | Yes |
| `g_IsMapEditor == 0` | required for `FUN_0069AE90` to update progress; editor skips callback work | `0x0069AE90` first branch | Yes for normal Skirmish |
| `ScenarioClass+0x34BD` | random scenario flag; `FUN_0069AE90` divides milestone input by 2 when set | `0x0069AE90`; final call in `0x00684620` passes `100` normal or `200` random | Conditional |
| `ProgressClass` current percent lane 0 | used by `FUN_0069AE90` to suppress non-advancing milestones | `0x0069AE90 -> FUN_00643E90(0)` | Yes |

## 3. Core Callback Logic

`FUN_0069AE90(param_1 = 0x00A8B238, param_2 = milestone)`:

1. If `g_IsMapEditor != 0`, return without progress work.
2. If `ScenarioClass+0x34BD != 0`, replace `milestone` with integer `milestone / 2`.
3. Read lane-0 current fraction through `FUN_00643E90(0)`.
4. Multiply current fraction by `100.0`.
5. Only if current percent is strictly less than requested milestone, call `FUN_00643C50(0, milestone, -1, -1)`.
6. Network progress-message side work may run for game modes `1..4`, but standard offline Skirmish (`5`) does not enter the multiplayer wait loops covered by `FUN_00684370`.

Evidence: decompile `0x0069AE90`; repaint gate report `PROGRESSCLASS_REPAINT_CADENCE_HWND_GHIDRA_REPORT.md`.

## 4. Milestone Ledger After First Renderer

This table is ordered by standard selected-map Skirmish execution after `0x00552D60`. "Visible effect" assumes a normal non-random map with monotonic lane-0 progress starting at `0`.

| Order | Value passed | Effective value | Owner / caller | Phase | Visible effect | Evidence | Active in YR? |
|---:|---:|---:|---|---|---|---|---|
| 1 | `3` | `3` | `ScenarioClass__Full_Init` | immediately after first renderer | advances | assembly `0x00687588..0x00687594`: call `0x00552D60`, push `3`, call `0x0069AE90` | Yes |
| 2 | `8` | `8` | `Init_Theater` | theater setup entry | advances | `0x00534A5B..0x00534A63`; called from `0x0068765B` | Yes |
| 3 | `6` | `6` | `Init_Theater` | theater MIX replacement branch | suppressed after `8` | `0x00534B5E..0x00534B65`; branch requires theater differs from cached `DAT_00822CF8` | Conditional, but non-advancing if reached after `8` |
| 4 | `12` | `12` | `Init_Theater` | theater palette/load setup | advances | `0x00534BE2..0x00534BE9` | Conditional on theater-change branch |
| 5 | dynamic `13..25` | same | `Init_Theater` | palette ramp loop | advances only when computed step changes | `0x00534D84..0x00534D9A`; clamps above `0x18` to `0x19`; skips unchanged step | Conditional on `DAT_00B054E0 > 0` |
| 6 | `25` | `25` | `Init_Theater` | theater finalization | advances or duplicate-suppressed if loop already reached `25` | `0x00534DB9..0x00534DC5` | Conditional on theater-change branch |
| 7 | `30` | `30` | `ScenarioClass__Full_Init` | post-theater core load | advances | `0x0068765B..0x00687667` | Yes |
| 8 | `31` | `31` | `ScenarioClass__Full_Init` | command bar read | advances | `0x00687694..0x0068769B` | Yes |
| 9 | `35` | `35` | `ScenarioClass__Full_Init` | scenario file/rules object setup | advances | `0x006876B1..0x006876B8` | Yes |
| 10 | `45` | `45` | `ScenarioClass__Full_Init` | rules processing / variable names | advances | `0x00687754..0x0068775B` | Yes |
| 11 | `50` | `50` | `ScenarioClass__Full_Init` | side mix init before Basic read | advances if side mix init succeeds | `0x00687833..0x00687847` | Yes, gated by successful `InitSideMixFiles` |
| 12 | `55` | `55` | `ScenarioClass__Read_INI_Basic` | map `[Basic]`, `[Header]`, lighting reads | advances | `0x0068AC93..0x0068ACA0` | Yes; called by `Full_Init @ 0x00687853` |
| 13 | `58` | `58` | `ScenarioClass__Read_INI_Basic` | player/house setup after Basic read | advances | `0x0068AD0A..0x0068AD34` | Yes |
| 14 | `60` | `60` | `ScenarioClass__Read_INI_Basic` | final Basic/map-editor gate section | advances | `0x0068AD4C..0x0068AD53` | Yes |
| 15 | `58` | `58` | `ScenarioClass__Full_Init` | outer post-`Read_INI_Basic` checkpoint | suppressed after inner `60` | `0x00687853..0x00687863` | Yes, but non-advancing |
| 16 | `60` | `60` | `ScenarioClass__Full_Init` | before map-section reader | suppressed after inner `60` | `0x006879ED..0x006879F4` | Yes, but duplicate |
| 17 | `63` | `63` | `Read_Map_Section_And_IsoMapPacks` | map base/theater field read | advances | `0x004AD00A..0x004AD011` | Yes; called by `Full_Init @ 0x006879FF` |
| 18 | `65` | `65` | `Read_Map_Section_And_IsoMapPacks` | theater tile/art section load | advances | `0x004AD0A8..0x004AD0AF` | Yes |
| 19 | `67` | `67` | `Read_Map_Section_And_IsoMapPacks` | celltag scan before IsoMapPack pipes | advances | `0x004AD332..0x004AD339` | Yes |
| 20 | `68` | `68` | `Read_Map_Section_And_IsoMapPacks` | after `IsoMapPack*` pipe decode | advances | `0x004AD70A..0x004AD716` | Yes |
| 21 | `69` | `69` | `Read_Map_Section_And_IsoMapPacks` | after `FUN_00546DA0` map helper | advances | `0x004AD743..0x004AD74F` | Yes |
| 22 | `70` | `70` | `ScenarioClass__Full_Init` | post-map-section, overlay pack boundary | advances | `0x00687A21..0x00687A28` | Yes |
| 23 | `72` | `72` | `ScenarioClass__Full_Init` | terrain/tiberium/radar boundary | advances | `0x00687A8F..0x00687A96` | Yes |
| 24 | `74` | `74` | `ScenarioClass__Full_Init` | units section boundary | advances | `0x00687AB1..0x00687AB8` | Yes |
| 25 | `76` | `76` | `ScenarioClass__Full_Init` | post-unit helper boundary | advances | `0x00687AD5..0x00687ADC` | Yes |
| 26 | `78` | `78` | `ScenarioClass__Full_Init` | buildings read boundary | advances | `0x00687AF4..0x00687AFB` | Yes |
| 27 | `82` | `82` | `ScenarioClass__Full_Init` | after optional `TMCJ4F.INI` branch / extra setup | advances | `0x00687B7B..0x00687B82` | Yes; optional file branch precedes it only if `g_GameMode == 5 && DAT_00A8ED91 != 0` |
| 28 | `86` | `86` | `ScenarioClass__Full_Init` | cell attributes init | advances | `0x00687B97..0x00687BA3` | Yes |
| 29 | `90` | `90` | `ScenarioClass__Full_Init` | beacon art init | advances | `0x00687BB7..0x00687BBE` | Yes |
| 30 | `93` | `93` | `ScenarioClass__Post_Map_Init` | post-map random units / selected-mode handoff | advances | `0x00686890` decompile; `FUN_0069AE90(0x5D)` after map-editor restoration | Conditional: reached from `Full_Init` when `g_GameMode != 0 && local flag == 0`; standard non-editor Skirmish path uses this branch in prior start-flow docs |
| 31 | `96` | `96` | `ScenarioClass__Full_Init` | after post-map init and display-chain cleanup | advances | `0x00687C00..0x00687C07` | Yes |
| 32 | `98` | `98` | `ScenarioClass__Full_Init` | final pre-return stage | advances | `0x00687C3F..0x00687C4C` | Yes |
| 33 | `100` | `100` | `ScenarioClass__Read_Scenario` | final scenario-read completion after `Full_Init` returns | advances to complete | `0x00684620` decompile: final call passes `(-random_flag & 100) + 100`; normal maps pass `100` | Yes |

## 5. Caller Exclusions

| Caller xref | Classification for this target | Evidence | Active in YR? |
|---|---|---|---|
| `FUN_00684370` | not a standard offline Skirmish after-first-render progress source | function immediately skips its wait-loop body for `g_GameMode == 5`; calls to final `100` are in the non-Skirmish branch | Conditional, No for standard offline Skirmish |
| `FUN_00648710` | multiplayer/queue wait progress resend, not standard offline Skirmish loading | decompile shows resend gated by `g_GameMode == 1 || g_GameMode == 2 || DAT_00A8DBA0 != 0`, queue UI, and network wait logic | Conditional, No for standard offline Skirmish |
| `FUN_00598960` | random map generator progress values, not normal selected-map after-first-render sequence | `ScenarioClass__Read_Scenario` calls it only in `ScenarioClass+0x34BD` random branch; its progress calls include `154..199` and are halved by `FUN_0069AE90` when scenario-loading flag is set | Conditional random-map path, outside normal selected-map core |
| `CCINIClass__Constructor @ 0x00599650` | function-boundary/xref neighborhood from random-map generation, not a direct selected-map post-render milestone owner for this slice | xref list plus `FUN_00598960` decompile context; no standard selected-map call chain from `Full_Init` after `0x00552D60` uses it as a milestone owner | No for this slice |
| pre-`0x00552D60` progress setup | out of scope for caller ledger after first renderer | `ScenarioClass__Read_Scenario @ 0x00684620` constructs progress UI and selects `PROGBARM.SHP` before first renderer | Yes before target anchor, not included |

## 6. Current Rust Implementation Status

| Surface | Current behavior | Delta |
|---|---|---|
| `src/app.rs` `GameScreen::Loading` | presents `GameScreen::Loading`, draws egui loading screen, then calls transition after present | no milestone ledger or pumpable load phases |
| `src/ui/main_menu.rs::draw_loading_screen` | text/egui loading UI with map-name-oriented surface | does not render native progress milestones or suppress duplicate/lower milestones |
| `src/app_transitions.rs::transition_to_in_game` | calls `app_init::load_map` synchronously | cannot expose the after-first-render milestone order to rendering |
| `src/app_init.rs::load_map` | loads assets/map as one synchronous application step | needs staged load or progress-event boundaries matching the ledger |

Evidence: Rust scan `src/app.rs:2051`, `src/app.rs:2177`, `src/ui/main_menu.rs:301`, `src/app_transitions.rs:32`, `src/app_init.rs:236`.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `FUN_0069AE90` callback gate | verified | decompile `0x0069AE90` | none for milestone gating |
| `ScenarioClass__Full_Init` post-render calls | verified | decompile `0x00686B20`; assembly contexts `0x00687594`, `0x00687667`, `0x0068769B`, `0x006876B8`, `0x0068775B`, `0x00687847`, `0x00687863`, `0x006879F4`, `0x00687A28`, `0x00687A96`, `0x00687AB8`, `0x00687ADC`, `0x00687AFB`, `0x00687B82`, `0x00687BA3`, `0x00687BBE`, `0x00687C07`, `0x00687C4C` | none for selected-map order |
| `Init_Theater` nested calls | verified | decompile `0x005349C0`; assembly contexts `0x00534A63`, `0x00534B65`, `0x00534BE9`, `0x00534D9A`, `0x00534DC5` | exact runtime value of `DAT_00B054E0` varies with palette table size but formula/gate verified |
| `Read_Map_Section_And_IsoMapPacks` nested calls | verified | decompile `0x004ACE70`; assembly contexts `0x004AD011`, `0x004AD0AF`, `0x004AD339`, `0x004AD716`, `0x004AD74F` | none for milestone values |
| `ScenarioClass__Read_INI_Basic` nested calls | verified | decompile `0x00689E90`; assembly contexts `0x0068ACA0`, `0x0068AD34`, `0x0068AD53` | none for milestone values |
| `ScenarioClass__Post_Map_Init` nested call | verified | decompile `0x00686890` | source of `Full_Init` local flag not fully named, but branch condition and caller site are verified |
| `ScenarioClass__Read_Scenario` final call | verified | decompile `0x00684620` | none for normal/random final values |
| multiplayer wait/resend xrefs | verified-excluded | decompile `0x00684370`, `0x00648710` | none for standard offline Skirmish |
| random-map generator xrefs | deferred | decompile `0x00598960` | random map loading deserves a separate targeted ledger if RandMap parity becomes the implementation target |

## 8. Open Questions - Final State

- `[RESOLVED] OQ-01 - What is the after-first-render anchor? -> Full_Init calls 0x00552D60, then immediately emits milestone 3.` (evidence: `0x00687588..0x00687594`)
- `[RESOLVED] OQ-02 - Which direct caller owns the main post-render sequence? -> ScenarioClass__Full_Init owns the ordered outer milestones from 3 through 98.` (evidence: `0x00686B20` decompile and assembly contexts)
- `[RESOLVED] OQ-03 - Which nested theater milestones are active? -> Init_Theater emits 8, conditional 6, 12, changed dynamic 13..25, and final 25.` (evidence: `0x005349C0`)
- `[RESOLVED] OQ-04 - Which nested map-pack milestones are active? -> Read_Map_Section_And_IsoMapPacks emits 63,65,67,68,69.` (evidence: `0x004ACE70`)
- `[RESOLVED] OQ-05 - Does Read_INI_Basic emit hidden-in-older-doc milestones? -> Yes, it emits 55,58,60 before the outer Full_Init 58 and 60 sites.` (evidence: `0x0068ACA0`, `0x0068AD34`, `0x0068AD53`)
- `[RESOLVED] OQ-06 - Are all calls visible redraws? -> No; FUN_0069AE90 only advances on strictly higher percent, so lower/duplicate calls are no-ops.` (evidence: `0x0069AE90`)
- `[RESOLVED] OQ-07 - Does standard offline Skirmish use multiplayer wait progress pulses? -> No; FUN_00684370 returns immediately for g_GameMode 5.` (evidence: `0x00684370`)
- `[RESOLVED] OQ-08 - What completes normal selected-map progress? -> ScenarioClass__Read_Scenario emits final 100 after Full_Init returns.` (evidence: `0x00684620`)
- `[RESOLVED] OQ-09 - How do random maps alter callback input? -> FUN_0069AE90 halves inputs when ScenarioClass+0x34BD is set; final call passes 200 so effective value is 100.` (evidence: `0x0069AE90`, `0x00684620`)
- `[DEFERRED] OQ-10 - Full random-map generator milestone ordering relative to its own visual surface.` (category: out-of-scope; reason: this report targets normal standard offline Skirmish selected-map progress after first verified renderer; next-step-if-pursued: run `/re-investigate random map loading progress milestones`)

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| After native first renderer, normal selected-map Skirmish emits milestones in this effective visible order: `3,8,12..25,30,31,35,45,50,55,58,60,63,65,67,68,69,70,72,74,76,78,82,86,90,93,96,98,100`, with conditional/suppressed entries as documented. | `0x00687594`, `0x005349C0`, `0x00689E90`, `0x004ACE70`, `0x00686890`, `0x00684620` | missing | app loading session, map-load orchestration, future loading renderer | expose native milestone events while keeping loading UI alive | start a normal offline Skirmish map and collect milestone events through completion | Do not jump from `3` to in-game or fake a smooth bar |
| Lower or duplicate milestones are real calls but visible no-ops: `6` after `8`, outer `58` after inner `60`, and later duplicate `60`; final `25` can also be duplicate if the theater loop already reached `25`. | `0x0069AE90`; call sites `0x00534B65`, `0x00687863`, `0x006879F4`, `0x00534DC5` | missing/unchecked | loading progress state and tests | store last effective percent and redraw only on strictly advancing effective values | replay the native ledger and assert no draw event for non-advancing calls | Do not redraw every callback blindly |
| `Read_INI_Basic` owns milestones `55`, `58`, and `60` inside the Full_Init span; older flattened ledgers that omit them are incomplete. | `0x0068ACA0`, `0x0068AD34`, `0x0068AD53`; `Full_Init` caller `0x00687853` | missing | staged map INI/basic load phase | split or instrument Basic-read phase so progress can reach 55/58/60 before map-pack work | load a stock Skirmish map and assert Basic-read milestones precede map-section `63` | Do not treat outer `Full_Init(58)` as the only Basic milestone |

Proposed tests:

- `loading_progress_standard_skirmish_selected_map_emits_verified_milestone_ledger`
- `loading_progress_nonadvancing_milestones_do_not_redraw`
- `loading_progress_read_ini_basic_milestones_precede_map_pack_milestones`

## 10. Negative Facts / Do Not Do

- Do not redraw on every `FUN_0069AE90` call; native suppresses non-advancing milestones.
- Do not omit `Read_INI_Basic` milestones `55`, `58`, and `60` from the ledger.
- Do not treat multiplayer wait/resend progress as active for standard offline Skirmish.
- Do not implement random-map generator values `154..199` as normal selected-map milestones.
- Do not reorder nested `Read_Map_Section_And_IsoMapPacks` milestones after outer `70`; native emits `63..69` before outer `70`.

## 11. Remaining Uncertainty

- Random-map generator milestones are intentionally deferred because they are not the normal selected-map after-first-render core.
- The exact source/name of the `Full_Init` local flag gating `Post_Map_Init` was not renamed, but the branch condition and the `Post_Map_Init(93)` call are verified.

## 12. Stale Docs / Follow-up Docs

Replace the milestone table in `docs/research/LOADING_PROGRESS_CALLBACK_VISIBLE_UI_GHIDRA_REPORT.md` where it summarizes "Side/basic setup" as only `50`, `58` with:

> Side/basic setup emits `50` in `ScenarioClass__Full_Init`, then `ScenarioClass__Read_INI_Basic` emits `55`, `58`, and `60` before returning. The following outer `FUN_0069AE90(58)` and later outer `FUN_0069AE90(60)` are real call sites but non-advancing visible no-ops on normal selected-map Skirmish because the current progress has already reached `60`.

## Sources

- Ghidra read-only decompile: `0x0069AE90`, `0x00686B20`, `0x005349C0`, `0x004ACE70`, `0x00686890`, `0x00684620`, `0x00684370`, `0x00648710`, `0x00598960`, `0x00689E90`.
- Ghidra read-only assembly contexts listed in the milestone ledger and coverage table.
- Prior docs: `LOADING_FIRST_RENDERER_00552D60_GHIDRA_REPORT.md`, `LOADING_PROGRESS_CALLBACK_VISIBLE_UI_GHIDRA_REPORT.md`, `PROGRESSCLASS_REPAINT_CADENCE_HWND_GHIDRA_REPORT.md`.
- Rust scan: `src/app.rs`, `src/ui/main_menu.rs`, `src/app_transitions.rs`, `src/app_init.rs`.

**Status:** COMPLETE for normal selected-map standard offline Skirmish callers after the first verified renderer; PARTIAL only for random-map generator progress, which is explicitly outside this slice.
