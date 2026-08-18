# Loading Read Scenario Pre-Full Init Progress Setup - Ghidra Research Report

**Address(es):** `0x00684620`, `0x00686730`, `0x00683AB0`, setup helpers `0x00642A60`, `0x00552A40`, `0x00552B10`, `0x00643E80`, `0x00642B10`, `0x00552CC0`, `0x00642C20`, `0x00552BE0`, `0x00642C80`, `0x00552C90`, `0x00642DF0`; boundary `0x00687588` / `0x00687594`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** standard offline Skirmish (`g_GameMode == 5`) progress/loading setup from `ScenarioClass::Start_Scenario` entry into `ScenarioClass::Read_Scenario`, through normal non-`.SED` `Read_Scenario_INI`, up to but not including first renderer `0x00552D60` and progress milestone `FUN_0069AE90(3)`.
**Non-Scope:** later milestone ledger after `3`, `0x00552D60` visual composition, `mmpb.shp` marker semantics, campaign loading briefing/text, random-map `.SED` generator internals, and full direct-draw/HWND repaint ownership.
**Confidence:** High for call order, setup values, defaults, and negative "no pre-Full_Init visible milestone callback" claim; Medium for the semantic name of `FUN_0072A9C0` base rect because this report only consumes its returned pair.
**Active in YR:** Yes. The path is live for standard offline Skirmish after Start success, when `Main_Game` passes `ScenarioClass+0x125C` to `ScenarioClass::Start_Scenario` and the selected map is an ordinary non-`.SED` scenario.

## 1. Target Question

Confirm whether visible progress callbacks happen before, during, or immediately around standard offline Skirmish `Read_Scenario` / pre-`Full_Init` setup, and identify the exact progress state, side, max, manager, asset, origin, and width values that must exist before the first native loading renderer `0x00552D60` and its following `FUN_0069AE90(3)`.

## 2. Non-Goals

- Do not enumerate later progress milestones after `FUN_0069AE90(3)`.
- Do not decode the `0x00552D60` LS background/marker/text composition.
- Do not re-open campaign `LSLoadMessage` rendering except to prove it is not a standard Skirmish pre-`Full_Init` setup input.
- Do not write Rust or alter in-repo docs.

## 3. Evidence Needed To Mark COMPLETE

- Prove the live standard Skirmish chain reaches `0x00684620`, `0x00686730`, and then `0x00686B20`.
- Prove the complete ordered setup call list before `Read_Scenario_INI` / `Full_Init`.
- Prove whether `FUN_0069AE90` is called before `0x00552D60`.
- Prove default max, lane count, HWND/direct-draw seed, side source, resource setup, `PROGBARM.SHP` selection, progress origin, and width override.
- Provide boundary rows for Rust: what exists before first render, what has not happened yet, and what must not be invented.

## 4. Stop Conditions

- Stop at the boundary `0x00687588` (`0x00552D60`) and `0x00687594` (`FUN_0069AE90(3)`).
- Stop if a branch enters campaign, LAN/multiplayer setup-message, or random `.SED` generation; mention only as non-target conditions.
- Stop before lower-level progress draw geometry except for fields seeded by this setup slice.

## 5. Core Logic And Call Order

For ordinary non-`.SED` standard offline Skirmish:

1. `ScenarioClass::Start_Scenario @ 0x00683AB0` starts the visible "LOADING" phase through `FUN_00721210("LOADING")` and `FUN_00720BB0`, then calls `ScenarioClass::Read_Scenario @ 0x00684620` at `0x00683D21`.
2. `Read_Scenario` copies the filename to a local buffer, sets `g_CurrentFrameCounter = 0`, writes `ScenarioClass+0x3598 = 1`, increments `g_MapEditorMode`, and detects `.SED` randomness by suffix compare. Standard ordinary Skirmish is the false branch: `ScenarioClass+0x34BD = 0`.
3. If `g_IsMapEditor == 0`, `Read_Scenario` configures the global ProgressClass and LoadProgressMgr before any scenario INI/full-init parsing.
4. For ordinary maps, `Read_Scenario` calls `ScenarioClass::Read_Scenario_INI @ 0x00686730`.
5. `Read_Scenario_INI` opens the scenario file through `CCFileClass`/`SHAPipe`, copies the filename into `ScenarioClass+0x125C`, then calls `ScenarioClass::Full_Init @ 0x00686B20` at `0x00686845`.
6. Early `Full_Init` performs non-campaign setup including `Create_Houses`, selected-mode start setup, and start assignment/mode startup. Then it constructs/reuses `LoadProgressMgr`, calls first renderer `0x00552D60` at `0x00687588`, and immediately calls `FUN_0069AE90(3)` at `0x00687594`.

No `FUN_0069AE90` visible progress milestone call was found in the target setup window before `0x00552D60`. The setup creates the progress surface state; the first verified progress advancement is milestone `3` after the first renderer returns.

## 6. Boundary Rows For Rust

| Boundary | Native state / action | Evidence | Rust requirement before first native LS frame |
|---|---|---|---|
| Start-to-read entry | `Start_Scenario` calls `FUN_00720BB0(LOADING)` then `Read_Scenario`. | `0x00683D0A..0x00683D21` | Loading job may enter a native loading phase before map parse, but this is not a progress milestone. |
| Progress object seeded | `FUN_00642A60` initializes ProgressClass max to double `100.0`, one lane for `g_GameMode == 5`, no HWND (`param_5 = 0`), progress lanes zeroed, enabled byte `+0x60 = 1`, current marker `+0x7C = -1`. | `0x006846EF..0x00684706`; `0x00642A60` | Create progress state before scenario parsing with max `100`, lane count `1`, no child HWND dependency, and value `0`. |
| Load manager created/registered | `LoadProgressMgr__Constructor` lazily allocates singleton `DAT_00ABC9BC` (100 bytes) only if null, then `FUN_00643E80` stores that pointer into ProgressClass `+0x04`. | `0x00684753..0x0068476B`; `0x00552A40`; `0x00643E80` | Loading job must own/reuse one loading manager/resource state and attach it to progress state before first render. |
| Layout base seeded | `FUN_00552B10` stores non-campaign background/layout rects in the manager; for Skirmish it sets `+0x1C/+0x20/+0x24/+0x28` from screen-centered `DAT_007F5BE0/BE4/BEC/BF0` dimensions. | `0x00684760`; `0x00552B10` | Build native loading layout from screen-size branch, not egui layout. |
| Side selected | `Read_Scenario` reads first node country at `*DAT_00A8DA78 + 0x4B`; sentinel `-3` becomes `0`; the country indexes `HouseTypeClass`, reads side at `+0xBC`, writes `ScenarioClass+0x34B8`, then `FUN_00642B10` stores the same value into ProgressClass `+0x80`. | `0x00684770..0x006847C9`; `0x00642B10` | Side must come from first launch/session node country before LS/progress assets draw. |
| Resource handles loaded | A second `LoadProgressMgr__Constructor` call reuses existing singleton, then `FUN_00552CC0` ensures `LOADMD.MIX` and `LOAD.MIX` handles and runs non-campaign loading resource setup. | `0x006847CE..0x006847D5`; `0x00552A40`; `0x00552CC0` | The same loading-manager asset state must back LS/progress assets and later map load handoff; do not create a separate facade asset manager. |
| Progress asset selected | `g_GameMode != 0` branch pushes `PROGBARM.SHP`, `0`, `0` to `FUN_00642C20`; campaign-only `SPLDBR.SHP` is skipped. | `0x006847DA..0x00684800`; `0x00642C20` | Select `PROGBARM.SHP` for standard Skirmish before the first renderer/milestone. |
| Explicit origin and flags stored | `FUN_00552BE0` computes Skirmish origin: base `+0x0C,+0x100` when `g_ScreenWidth == DAT_007F5BE0`, else base `+0x10,+0x141`; `FUN_00642C80` stores the explicit origin, null setup-message pointer, and non-campaign flags `+0x70=1`, `+0x71=1`. | `0x00684805..0x00684825`; `0x00552BE0`; `0x00642C80` | Store explicit native progress origin/flags before any milestone draw; do not use auto-center branch. |
| Width override stored | `FUN_00552C90` returns `0x146` for `g_ScreenWidth == DAT_007F5BE0`, else `0x196`; `FUN_00642DF0` writes this to ProgressClass `+0x78`. | `0x0068482A..0x00684832`; `0x00552C90`; `0x00642DF0` | Store native progress width override before first progress draw. |
| Normal INI boundary | Ordinary maps call `Read_Scenario_INI`, which opens file, copies filename into `ScenarioClass+0x125C`, then calls `Full_Init`. | `0x006849C3..0x006849C9`; `0x00686730`; `0x00686845` | A pumpable Rust loader should not call map parsing before native loading/progress state exists. |
| First render/progress boundary | `Full_Init` calls `0x00552D60`, then immediately pushes `3` and calls `FUN_0069AE90`. | `0x00687581..0x00687594` | First visible native LS frame must have all above setup ready; milestone `3` is applied after the LS renderer, not before. |

## 7. Setup Values And Fields

| Field / value | Writer | Value in standard offline Skirmish | Active in YR? | Why it matters |
|---|---|---|---|---|
| `ScenarioClass+0x3598` | `0x00684620` | `1` during read/load, cleared at end/failure | Yes | Marks scenario loading state around setup/read. |
| `ScenarioClass+0x34BD` | `0x00684620` | `0` for ordinary non-`.SED` maps | Yes | Prevents random-map milestone halving in `FUN_0069AE90`. |
| ProgressClass max `+0x48/+0x4C` | `0x00642A60` | double `100.0` (`0x00000000,0x40590000`) | Yes | Milestone integers are percent-like against max 100. |
| ProgressClass lane count `+0x61` | `0x00642A60` | `1` because `g_GameMode == 5` takes the campaign/skirmish one-lane branch | Yes | Rust should not use player-count lanes for offline Skirmish. |
| ProgressClass HWND `+0x64` | `0x00642A60` | `0` | Yes | Standard map load is set up for direct draw fallback, not child control dependency. |
| ProgressClass manager pointer `+0x04` | `0x00643E80` | `DAT_00ABC9BC` | Yes | Later direct-draw progress uses this manager state. |
| ProgressClass selected side `+0x80` | `0x00642B10` | side from first node country -> HouseType `+0xBC` | Yes | Side-dependent loading assets/palette inputs rely on this being ready. |
| ProgressClass SHP `+0x54` | `0x00642C20` | `PROGBARM.SHP` handle/pointer | Yes | Progress uses Skirmish progress chrome, not campaign `SPLDBR`. |
| ProgressClass text pointer `+0x50` | `0x00642C80` | null for standard Skirmish setup | Yes | No `LSLoadMessage` or LAN setup message is seeded here. |
| ProgressClass origin `+0x68/+0x6C` | `0x00642C80` | explicit `0x00552BE0` result | Yes | Native progress placement is not egui or auto-centered. |
| ProgressClass flags `+0x70/+0x71` | `0x00642C80` | both `1` | Yes | Non-campaign draw path options are enabled. |
| ProgressClass width `+0x78` | `0x00642DF0` | `0x146` or `0x196` | Yes | Native clipped fill row width is pre-seeded. |

## 8. Visible Progress Callback Answer

| Question | Answer | Evidence | Confidence |
|---|---|---|---|
| Any `FUN_0069AE90` before `Read_Scenario` setup? | No in the scoped standard Skirmish chain. `Start_Scenario` switches to "LOADING", then calls `Read_Scenario`; it does not call `FUN_0069AE90` in this interval. | `0x00683D0A..0x00683D21`; `0x00683AB0` decompile | High |
| Any `FUN_0069AE90` during setup before `Read_Scenario_INI`? | No. Setup calls configure `ProgressClass`/`LoadProgressMgr`, side, assets, origin, and width, then branch to `Read_Scenario_INI`. | `0x00684620` decompile; setup assembly `0x006846EF..0x00684832`; normal branch `0x006849C9` | High |
| Any visible progress callback inside `Read_Scenario_INI` before `Full_Init`? | No. It opens the map, copies filename to `ScenarioClass+0x125C`, then calls `Full_Init`. | `0x00686730`; call at `0x00686845` | High |
| First verified visible progress milestone around setup? | `FUN_0069AE90(3)`, immediately after `0x00552D60`, inside early `Full_Init`. | `0x00687588..0x00687594` | High |
| Does pre-`Full_Init` setup still have player-visible consequences? | Yes, because it decides the native loading/progress assets, side, layout, direct-draw fallback, and initial zero state used by the first renderer and milestone. | setup helpers above plus prior progress/first-renderer reports | High |

## 9. Current Rust Implementation Status

Current Rust still flips into `GameScreen::Loading` and draws egui before the native-equivalent scenario loading state exists:

- `src/app.rs` sets `GameScreen::Loading { map_name }` from Skirmish launch paths.
- `src/app.rs` draws `main_menu::draw_loading_screen` for `GameScreen::Loading`.
- `src/app.rs` then calls `app_transitions::transition_to_in_game` after present.
- `src/app_transitions.rs::transition_to_in_game` calls `app_init::load_map` synchronously.
- `src/ui/main_menu.rs::draw_loading_screen` renders egui text/panel content instead of the pre-seeded native progress/LS state.

Rust delta: missing pre-map-parse loading setup state, missing native progress object, missing first-node side selection for loading, missing shared loading resource manager, missing `PROGBARM.SHP` setup, missing explicit origin/width seed, and wrong ordering because Rust shows a fallback loading surface before the native setup boundary.

## 10. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `ScenarioClass::Start_Scenario @ 0x00683AB0` to `Read_Scenario` | verified | `0x00683D0A..0x00683D21` | no later post-read success path claimed |
| `Read_Scenario @ 0x00684620` ordinary Skirmish setup | verified | decompile; assembly `0x006846EF..0x00684832` | random `.SED` generator out-of-scope |
| `ProgressClass` init helper `0x00642A60` | verified | decompile | full ProgressClass layout outside setup fields out-of-scope |
| `LoadProgressMgr` lazy constructor `0x00552A40` | verified | decompile | resource teardown outside target boundary |
| manager layout helper `0x00552B10` | verified | decompile | exact semantic names of width constants not renamed |
| side setter `0x00642B10` | verified | decompile and caller | none |
| resource setup `0x00552CC0` | verified | decompile | exact palette/LS art composition belongs to first-renderer report |
| `PROGBARM.SHP` selection `0x00642C20` | verified | decompile and caller assembly | actual fill draw geometry belongs to progress geometry report |
| origin helper `0x00552BE0` | verified | decompile | base provider `FUN_0072A9C0` not fully decoded here |
| origin/options setter `0x00642C80` | verified | decompile | later text fallback not runtime-captured |
| width helper/setter `0x00552C90` / `0x00642DF0` | verified | decompile | none |
| `Read_Scenario_INI @ 0x00686730` pre-`Full_Init` | verified | decompile; assembly `0x00686845` | map parser internals out-of-scope |
| first renderer/milestone boundary | verified | `0x00687588..0x00687594` | later milestone owners belong to other swarm slots |
| current Rust comparison | touched-not-exhausted | `rg` scan of `src/app.rs`, `src/app_transitions.rs`, `src/app_init.rs`, `src/ui/main_menu.rs` | exact future module design out-of-scope |

## 11. Open Questions - Final State

- `[RESOLVED] OQ-01 - Is this path active in standard offline Skirmish? -> Yes; `g_GameMode == 5` enters `Start_Scenario -> Read_Scenario -> Read_Scenario_INI -> Full_Init` for ordinary non-`.SED` maps.` (evidence: `0x00683D21`, `0x00684620`, `0x00686730`)
- `[RESOLVED] OQ-02 - Does `Start_Scenario` call the progress milestone callback before `Read_Scenario`? -> No; it calls the LOADING screen helpers and then `Read_Scenario`, with no `FUN_0069AE90` call in that interval.` (evidence: `0x00683D0A..0x00683D21`)
- `[RESOLVED] OQ-03 - Does `Read_Scenario` setup itself call `FUN_0069AE90` before `Read_Scenario_INI`? -> No; setup calls only initialize progress/manager/asset/origin state before branching to `Read_Scenario_INI`.` (evidence: `0x006846EF..0x00684832`, `0x006849C9`)
- `[RESOLVED] OQ-04 - Does `Read_Scenario_INI` emit visible progress before `Full_Init`? -> No; it opens/loads the INI and immediately calls `Full_Init` after copying filename.` (evidence: `0x00686730`, `0x00686845`)
- `[RESOLVED] OQ-05 - What is the first verified progress milestone? -> `FUN_0069AE90(3)`, immediately after first renderer `0x00552D60`.` (evidence: `0x00687588..0x00687594`)
- `[RESOLVED] OQ-06 - What max/range is seeded before first render? -> ProgressClass max is double `100.0`; lane count is `1`; initial lane value is zeroed.` (evidence: `0x006846EF..0x00684706`, `0x00642A60`)
- `[RESOLVED] OQ-07 - Is a child HWND seeded for standard Skirmish progress? -> No; the setup passes `0`, and `0x00642A60` stores `ProgressClass+0x64 = 0`.` (evidence: `0x006846FB..0x00684706`, `0x00642A60`)
- `[RESOLVED] OQ-08 - How is loading side selected before first render? -> first session node country at `*DAT_00A8DA78+0x4B`, `-3 -> 0`, HouseType `+0xBC`, stored to `ScenarioClass+0x34B8` and ProgressClass `+0x80`.` (evidence: `0x00684770..0x006847C9`, `0x00642B10`)
- `[RESOLVED] OQ-09 - Which progress SHP is seeded? -> `PROGBARM.SHP` for non-campaign; `SPLDBR.SHP` is campaign-only in this setup branch.` (evidence: `0x006847DA..0x00684800`)
- `[RESOLVED] OQ-10 - What origin and width are seeded before first render? -> explicit origin from `0x00552BE0` and width `0x146/0x196` from `0x00552C90`, stored through `0x00642C80` and `0x00642DF0`.` (evidence: `0x00552BE0`, `0x00552C90`, `0x00684805..0x00684832`)
- `[RESOLVED] OQ-11 - Is the `LSLoadMessage`/campaign text setup active here? -> No for standard Skirmish; setup message pointer is null because the formatted message branch is `g_GameMode == 4 && DAT_00B779C4 != 0`, and campaign `LS*` fields are read only in the `g_GameMode == 0` branch.` (evidence: `0x00684620`)
- `[DEFERRED] OQ-12 - Exact runtime content of any later non-campaign text fallback from `*DAT_00A8DA78`.` (category: needs-runtime-debugger; reason: this report proves the pointer is not seeded by setup and does not affect the pre-first-render boundary; next-step-if-pursued: trace `0x00643AE0` text pointer during live Skirmish load)
- `[DEFERRED] OQ-13 - Exact `FUN_0072A9C0` base rect provider semantics.` (category: requires-different-system-context; reason: `0x00552BE0` uses the returned base pair, which is sufficient for this boundary; next-step-if-pursued: decode LS layout provider alongside first-renderer composition)

## 12. Visual/UI Composition Ledger

This report covers the pre-composition setup, not the pixels inside `0x00552D60`.

| Order | Function / address | Condition / flag proof | Asset / frame | Rect / anchor | Palette / convert | Active for target? | Role |
|---:|---|---|---|---|---|---|---|
| 0 | `Start_Scenario -> FUN_00720BB0` | immediately before `Read_Scenario` | string/state `"LOADING"` | shell/display phase, not ProgressClass draw | n/a | yes | loading phase entry |
| 1 | `0x00642A60` | `g_IsMapEditor == 0`, `g_GameMode == 5` | none | max/lane/direct-draw seed | n/a | yes | progress state initialization |
| 2 | `0x00552A40`, `0x00643E80` | setup path | none | manager pointer attached to ProgressClass | n/a | yes | manager/resource owner |
| 3 | `0x00642B10` | side from first node country | none | `ScenarioClass+0x34B8`, ProgressClass `+0x80` | n/a | yes | side input |
| 4 | `0x00552CC0`, `0x00642C20` | non-campaign branch | `PROGBARM.SHP`, frame later | progress asset handle | later ProgressClass convert | yes | progress asset setup |
| 5 | `0x00552BE0`, `0x00642C80`, `0x00552C90`, `0x00642DF0` | non-campaign explicit path | none | origin base `+0x0C,+0x100` or `+0x10,+0x141`; width `0x146/0x196` | n/a | yes | progress layout setup |
| 6 | `0x00552D60` | after `Full_Init` early non-campaign setup | LS art, not decoded here | first renderer | `MPLS*.PAL` per sibling report | yes | first native LS render |
| 7 | `FUN_0069AE90(3)` | immediately after `0x00552D60` | `PROGBARM.SHP` via ProgressClass | uses seeded origin/width | ProgressClass draw path | yes | first progress milestone |

Asset role matrix:

| Asset | Loaded/seeded before first render | Drawn before first render | Visible in target | Role | Evidence |
|---|---:|---:|---:|---|---|
| `PROGBARM.SHP` | yes | no, first draw after `FUN_0069AE90(3)` | yes after renderer | progress chrome/fill | `0x006847F2..0x00684800`; `0x00687594` |
| `SPLDBR.SHP` | no for Skirmish | no | no | campaign progress asset | `0x006847E1..0x006847F0` |
| `LOADMD.MIX` / `LOAD.MIX` | yes, manager handles ensured | support only | input-only | loading resources | `0x00552CC0` |
| `ls640/ls800<country>.shp` | loaded by manager resource path / first-renderer family | first-renderer report owns exact draw | yes | loading background | sibling `LOADING_FIRST_RENDERER_00552D60_GHIDRA_REPORT.md` |
| `PUDLGBG*.SHP` | not part of this standard Skirmish pre-`Full_Init` setup | no | no in this boundary | separate WM_PAINT mode-2 branch | sibling mode-2 reports |

## 13. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| `Read_Scenario` seeds the native progress manager before ordinary map parsing/full init, with max `100`, one lane, no HWND, zero value, side, resource manager, `PROGBARM.SHP`, explicit origin, and width. | `0x006846EF..0x00684832`; helpers `0x00642A60`, `0x00552A40`, `0x00642B10`, `0x00642C20`, `0x00552BE0`, `0x00552C90` | missing; Rust draws egui loading before native setup and blocks in `load_map` | `src/app.rs`, `src/app_transitions.rs`, future loading job/session state | Build a pre-map-parse loading setup phase that creates native loading/progress state before parsing begins. | Start standard Skirmish; first native loading frame already has side/progress state and no egui fallback. Proposed test: `skirmish_loading_setup_precedes_map_parse`. | Do not create a visual facade that is separate from the loading manager/resource state used by the job. |
| No visible `FUN_0069AE90` milestone fires before first renderer; milestone `3` is immediately after `0x00552D60`. | `0x00683D21`, `0x006849C9`, `0x00686845`, `0x00687588..0x00687594` | mismatch risk if Rust advances progress before drawing LS art | loading app-loop/pump sequencing | Render first LS composition after setup, then apply progress milestone `3`; keep pre-render state at value `0`. | Event trace order is `setup -> first_ls_render -> milestone_3`, not `setup -> milestone_3 -> first_ls_render`. Proposed test: `skirmish_loading_first_render_precedes_milestone_3`. | Do not show an initial progress fill based on `3` before LS background composition. |
| Standard Skirmish setup does not seed campaign/LAN loading text; the setup message pointer is null and side/text inputs come from session/node data later if needed. | `0x00684620`; `0x00642C80`; sibling text split report | mismatch: `src/ui/main_menu.rs` shows `"Loading..."`, map name, and status prose | `src/ui/main_menu.rs`, native loading renderer | Remove invented egui/map-name/status text from the standard Skirmish parity loading surface. | Loading stock Skirmish has native LS/progress assets, no Rust explanatory panel or map-name overlay. Proposed test: `skirmish_loading_has_no_egui_or_map_name_overlay`. | Do not use `LSLoadMessage`, `LSLoadBriefing`, `Briefing`, `UIName`, or LAN setup message for offline Skirmish. |

## 14. Negative Facts / Do Not Do

- Do not emit or draw milestone `3` during `Read_Scenario` setup; it occurs after `0x00552D60`.
- Do not initialize offline Skirmish as multi-lane/player-count progress; `g_GameMode == 5` uses one lane in `0x00642A60`.
- Do not depend on a child HWND progress control for standard map load setup; `ProgressClass+0x64` is seeded as zero.
- Do not use `SPLDBR.SHP`, campaign `LSLoadMessage`, or LAN setup message for standard offline Skirmish.
- Do not auto-center the Skirmish progress row through the `(-1,-1)` branch of `0x00642C80`; `Read_Scenario` passes explicit coordinates.
- Do not show map filename/display name or egui explanatory loading prose as a parity surface.
- Do not use `PUDLGBG*` as the first standard Skirmish `Read_Scenario`/`Full_Init` loading renderer; that is a separate `WM_PAINT` mode-2 branch.

## 15. Remaining Uncertainty

- Exact runtime text drawn later from the non-campaign fallback pointer remains deferred to a runtime/composition pass.
- Exact semantic name and full layout contract of `FUN_0072A9C0` base rect provider remains deferred; this report verifies only the offsets added to its returned base pair.
- Later milestone values/owners after `FUN_0069AE90(3)` are owned by sibling swarm slots, not this report.

## 16. Stale Docs / Follow-up Docs

No existing report needs replacement for this slot beyond reinforcing prior corrections. If updating broader synthesis wording, use:

> For standard offline Skirmish, `ScenarioClass::Read_Scenario @ 0x00684620` configures the native ProgressClass/LoadProgressMgr before ordinary scenario parsing. This setup seeds max `100`, one lane, no progress HWND, first-node side, `PROGBARM.SHP`, explicit progress origin/width, and then calls `Read_Scenario_INI -> Full_Init`. It does not emit a visible `FUN_0069AE90` milestone before first renderer `0x00552D60`; the first verified progress milestone is `FUN_0069AE90(3)` immediately after that renderer.

## Sources

- Ghidra decompiled/read-only: `0x00683AB0`, `0x00684620`, `0x00686730`, `0x00686B20`, `0x00642A60`, `0x00552A40`, `0x00552B10`, `0x00643E80`, `0x00642B10`, `0x00552CC0`, `0x00642C20`, `0x00552BE0`, `0x00642C80`, `0x00552C90`, `0x00642DF0`, `0x0069AE90`.
- Ghidra assembly context/read-only: `0x00683D0A..0x00683D21`, `0x006846EF..0x00684832`, `0x006849C9`, `0x00686845`, `0x00687588..0x00687594`.
- Prior reports referenced: `LOAD_PROGRESS_MANAGER_SETUP_GHIDRA_REPORT.md`, `LOADING_FIRST_RENDERER_00552D60_GHIDRA_REPORT.md`, `SKIRMISH_START_TO_LOADING_SCREEN_ACTIVATION_GHIDRA_REPORT.md`, `LOADING_PROGRESS_CALLBACK_VISIBLE_UI_GHIDRA_REPORT.md`, `LSLOADMESSAGE_SKIRMISH_LOADING_TEXT_SPLIT_GHIDRA_REPORT.md`, `PROGRESSCLASS_REPAINT_CADENCE_HWND_GHIDRA_REPORT.md`.
- Current Rust scan: `src/app.rs`, `src/app_transitions.rs`, `src/app_init.rs`, `src/ui/main_menu.rs`, `src/skirmish_launch.rs`.

**Status:** COMPLETE for the scoped pre-`Full_Init` setup boundary.
