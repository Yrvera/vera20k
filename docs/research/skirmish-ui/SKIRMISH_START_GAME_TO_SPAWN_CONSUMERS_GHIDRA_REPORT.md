# Skirmish Start Game To Spawn Consumers - Ghidra Research Report

**Address(es):** `0x006ACEE0`, `0x006AE2C0`, `0x00686B20`, `0x00687F10`, `0x005D6BE0`, `0x005EE9D0`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** offline Skirmish dialog `0x102` Start Game command `0x617` through the committed session globals/node records and the immediate scenario/house/start-assignment consumers that read them.  
**Non-Scope:** detailed MCV/unit placement formulas, distance scoring, fallback random start generation internals, complete game-mode family behavior beyond the standard Battle-mode consumer slot, and tactical gameplay after scenario init.  
**Confidence:** High for the Start branch, committed globals, `ScenarioClass::Full_Init`, `ScenarioClass::Create_Houses`, and Battle-mode preassignment reader; Medium for names on legacy helper labels where Ghidra labels conflict with observed offsets.  
**Active in YR:** Yes. Evidence: the path is in the non-campaign branch of `ScenarioClass__Full_Init @ 0x00686B20`; offline Skirmish uses `g_GameMode == 5`, which follows the same non-campaign house/start setup branch and has an explicit Skirmish-only check later in the same initializer.

## 1. Overview

Pressing Start Game in offline Skirmish does not directly spawn anything. `FUN_006ACEE0 @ 0x006ACEE0` validates the dialog, commits selected map/options/slot data into global session arrays and `DAT_00A8DA78` node records, then lets `FUN_006AE2C0 @ 0x006AE2C0` return true.

The first spawn/setup consumers are in scenario initialization, not in the shell. `ScenarioClass__Full_Init @ 0x00686B20` clears the scenario start table, reads map waypoints, creates houses from the session/node records via `ScenarioClass__Create_Houses @ 0x00687F10`, then calls the selected mode object's vtable `+0x80`; for the standard Battle vtable this lands at code beginning `0x005D6BE0`, which copies explicit start choices from `House+0x16058` into `ScenarioClass+0x1180`. `ScenarioClass__AssignStartingPoints @ 0x005EE9D0` then consumes `ScenarioClass+0x1180`.

## 2. Committed UI Data

| Committed data | Writer from Start branch | Immediate consumer | Active in YR |
|---|---|---|---|
| selected map token/index mirrors `DAT_00A8B3C4/3C8` from `DAT_00A8B250/254` | `0x006AD34B..0x006AD36B` | scenario/map load selection path before/around `ScenarioClass__Full_Init`; exact loader entry not expanded here | Yes; non-campaign scenario load |
| active AI count `DAT_00A8B274` | active row count at `0x006ACFBD..0x006AD052` | `ScenarioClass__Create_Houses` AI loop limit | Yes; `0x00688112` reads `DAT_00A8B274` |
| local node record vector `DAT_00A8DA78/84` | allocation/insertion at `0x006AD647..0x006AD6F6` | `ScenarioClass__Create_Houses` human house loop; Battle preassignment/observer checks | Yes; `0x00687F10`, `0x005D6BE0` |
| node `+0x4B` country | `0x006AD677` from `DAT_00A8B3AC` after random assignment | `Create_Houses` house type constructor lookup | Yes; `0x00688018` / `0x006880A7` |
| node `+0x53` color priority | `0x006AD67E` from `DAT_00A8B394` | `Create_Houses` ordering and `HouseClass__Set_Credits_And_Color` | Yes; `0x00687FA6..0x006880FC` |
| node `+0x5B` local start selection | `0x006AD685` from `DAT_00A8B39C` | `Create_Houses` writes `House+0x16058`; Battle vtable `+0x80` reads `House+0x16058` | Yes; `0x006880FC`, `0x005D6C12` |
| node `+0x63` local team/adjunct selection | `0x006AD68C` from `DAT_00A8B3A4` | `Create_Houses` writes `House+0x1605C`; downstream team/alliance consumer not expanded | Yes; `0x00688107` |
| node `+0x6B = -1` | `0x006AD693` | `Create_Houses` observer marker and Battle preassignment observer branch | Yes/Conditional; observer role only when value remains `-1`, evidence `0x0068811C`, `0x005D6C05..0x005D6C36` |
| AI arrays `DAT_00A8B29C/B2BC/B2DC/B2FC/B27C` | `0x006AD3C1..0x006AD4E6` | `Create_Houses` AI house loop | Yes; `0x00688112..0x006882D5` |
| top-level credits/speed/unit count and checkbox mirrors | `0x006AD703..0x006AD889` | credits consumed in `Create_Houses`; other option consumers are runtime systems outside this slot | Yes; credits read as `DAT_00A8B25C` at `0x006880F7` and `0x00688189` |

## 3. Consumer Chain

1. `FUN_006AE2C0 @ 0x006AE2C0` is the modal Skirmish setup loop. It stores a local result pointer in dialog user data, runs until that local is `0x617` or `0x5C0`, and returns `local == 0x617`. Active in YR: Yes; this is the offline Skirmish dialog launcher.

2. `FUN_006ACEE0 @ 0x006ACEE0` handles `0x617` only when notification is `0`. It disables the button, validates active row count, selected map capacity, minimum total players, same-team mode constraint, and selected-mode vtable `+0x14` acceptance. Only after that does it commit arrays/node records and write `0x617` to the modal result pointer. Active in YR: Yes; standard offline Skirmish Start.

3. `SessionClass__ProcessRandomAssignments @ 0x0069B8C0` is called before the shell exits. It resolves random country/color values both in node records and AI arrays, so `Create_Houses` generally reads resolved values rather than raw `-2` random sentinels. Active in YR: Yes; call at `0x006AD6F9`.

4. `ScenarioClass__Full_Init @ 0x00686B20` is the first verified non-shell consumer stage. In the `g_GameMode != 0` branch it clears `ScenarioClass+0x1180..0x11C0` to `-1`, reads map waypoints with `FUN_0068BDC0`, calls `ScenarioClass__Create_Houses`, calls the selected mode object's vtable `+0x80`, and then either calls `ScenarioClass__AssignStartingPoints` when `DAT_00A8B244 == 2` or the mode vtable `+0x84` otherwise. Active in YR: Yes; offline Skirmish is non-campaign and the same function later checks `g_GameMode == 5`.

5. `ScenarioClass__Create_Houses @ 0x00687F10` consumes `DAT_00A8DA78/84`, `DAT_00A8B274`, the AI arrays, and `DAT_00A8B25C`. It creates human and AI `HouseClass` instances, sets country/color/credits/name/human flag/difficulty, writes start/team adjunct fields into `House+0x16058` and `House+0x1605C`, and sets `g_PlayerPtr` for the local human. Active in YR: Yes; called unconditionally in the non-campaign init branch.

6. Standard Battle-mode vtable `+0x80` is the immediate explicit-start consumer. The Battle vtable at `0x007EE184` has its `+0x80` entry pointing to code at `0x005D6BE0`; assembly context shows it calls `ScenarioClass__Gather_Start_Positions`, loops `g_HouseClass_Array`, reads each non-special house's `House+0x16058`, skips `-2`, and writes the house index into `ScenarioClass+0x1180 + start_index*4`. Active in YR: Yes for Battle/ManBattle style Skirmish modes; conditional for other mode objects because their vtable targets may differ.

7. `ScenarioClass__AssignStartingPoints @ 0x005EE9D0` consumes `ScenarioClass+0x1180`: it builds a 16-byte occupied array from that table, assigns human houses first and AI houses second, and uses `ScenarioClass__Gather_Start_Positions @ 0x00688380` for the start-cell list. Active in YR: Yes when `DAT_00A8B244 == 2`; otherwise `ScenarioClass__Full_Init` dispatches selected-mode vtable `+0x84`.

## 4. Field Correction From This Slot

Older Skirmish start-position docs that describe `House+0x1605C` as the field consumed for start preassignment are stale for the Battle-mode consumer. The verified reader at `0x005D6C12` loads `House+0x16058`, then `0x005D6C2F` writes into `ScenarioClass+0x1180 + start_index*4`. `House+0x1605C` is still written by `Create_Houses`, but this slot did not find it used by the immediate Battle-mode start preassignment reader.

Active in YR: Yes for standard Battle/ManBattle-style offline Skirmish. Evidence: vtable `0x007EE184 + 0x80 -> 0x005D6BE0`, assembly context `0x005D6BEC..0x005D6C2F`, and `ScenarioClass__Full_Init @ 0x00686B20` calls selected mode vtable `+0x80` before start assignment in the non-campaign branch.

## 5. Current Rust Implementation Status

Rust currently carries only a narrow subset of the native launch contract:

| Rust field/path | Current use | Native equivalent still missing |
|---|---|---|
| `SkirmishSettings.selected_map_idx` | `src/app.rs:414` selects map file for loading | native selected map token/index mirrors plus out-of-range clamp |
| `SkirmishShellState` opponent vector | `src/ui/skirmish_shell/state.rs:20..64` stores enabled/country/color/start/team | native seven AI rows with kind/country/color/start/team/difficulty arrays |
| `launch_settings` | `src/ui/skirmish_shell/state.rs:70..86` collapses to one AI country and player start | no per-row arrays, random resolution, or launch table |
| `start_selected_skirmish` | `src/app.rs:411..420` enters Loading with map name | no validation/session acceptance or packed session/node records |
| `seed_skirmish_opening_if_needed` | `src/app_skirmish.rs:25..86` swaps chosen start to index 0, reorders houses by side, spawns two MCVs | no house creation from node records, no `ScenarioClass+0x1180` style preassignment table, no all-slot AI/difficulty/team handling |
| `GameOptions` | `src/sim/game_options.rs:14..51` has options fields and defaults | Start UI does not feed most of the Start branch mirrors into game start |

The state fields that must eventually feed game start for parity are: selected map token/index, game mode/category, local player country/color/start/team/name/observer role, every enabled AI row's kind/country/color/start/team/difficulty, starting credits, game speed, unit count, Short Game, Super Weapons, Build Off Ally, MCV Repacks, Crates, and the forced launch flags currently written at `0x006AD88F..0x006AD8A4`.

## 6. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| dialog `0x102` modal exit | verified | `FUN_006AE2C0 @ 0x006AE2C0` | none |
| Start command packing | verified | `FUN_006ACEE0 @ 0x006ACEE0` | exact CSF text for validation messages out of scope |
| random assignment resolution | verified | `SessionClass__ProcessRandomAssignments @ 0x0069B8C0` | RNG seed/source out of scope |
| non-campaign scenario init consumer order | verified | `ScenarioClass__Full_Init @ 0x00686B20` | exact caller from main state machine not expanded |
| map waypoint load before assignment | verified | `FUN_0068BDC0 @ 0x0068BDC0` | waypoint parsing formula already outside scope |
| house creation consumer | verified | `ScenarioClass__Create_Houses @ 0x00687F10` | downstream production/base setup outside scope |
| Battle-mode vtable `+0x80` start preassignment | verified | vtable memory `0x007EE184 + 0x80`; assembly `0x005D6BE0..0x005D6C68` | other mode objects' `+0x80` implementations not expanded |
| start assignment consumer | touched-not-exhausted | `ScenarioClass__AssignStartingPoints @ 0x005EE9D0` | placement/distance/fallback formulas deliberately out of scope |
| `House+0x1605C` downstream consumer | deferred | this slot verified it is not the Battle preassignment field | separate team/alliance consumer trace |

## 7. Open Questions - Final State

[RESOLVED] OQ-1 - Does the Start button directly spawn units? No. It writes session/node/global data and exits the dialog; scenario init consumes the data later. Evidence: `0x006ACEE0`, `0x006AE2C0`, `0x00686B20`.

[RESOLVED] OQ-2 - Which field feeds explicit start preassignment in the standard Battle-mode consumer? `House+0x16058`, copied from node `+0x5B` / local `DAT_00A8B39C` / AI `DAT_00A8B2DC`. Evidence: `0x006AD685`, `0x006880FC`, `0x005D6C12`, `0x005D6C2F`.

[RESOLVED] OQ-3 - Which table does start assignment consume? `ScenarioClass+0x1180`, initialized to `-1`, populated by the selected mode preassignment method, then read by `ScenarioClass__AssignStartingPoints`. Evidence: `0x00686B20`, `0x005D6C2F`, `0x005EE9D0`.

[RESOLVED] OQ-4 - Does `Create_Houses` consume the AI slot arrays directly? Yes: country/color/start/team/difficulty arrays are read from `DAT_00A8B29C/B2BC/B2DC/B2FC/B27C` in the AI house loop. Evidence: `0x00688112..0x006882D5`.

[DEFERRED] OQ-5 - What is the gameplay placement formula after a house has an assigned start cell? Category: out-of-scope. Reason: user explicitly excluded gameplay spawn placement formulas beyond naming immediate consumers.

[DEFERRED] OQ-6 - Which systems consume `House+0x1605C` after `Create_Houses`? Category: requires-different-system-context. Reason: this slot's bounded consumer trace found the start preassignment reader at `House+0x16058`; `House+0x1605C` should be traced as a team/alliance/adjunct consumer separately.

## Sources

- Ghidra decompiled/read-only: `FUN_006ACEE0 @ 0x006ACEE0`, `FUN_006AE2C0 @ 0x006AE2C0`, `FUN_006AE3F0 @ 0x006AE3F0`, `SessionClass__ProcessRandomAssignments @ 0x0069B8C0`, `ScenarioClass__Full_Init @ 0x00686B20`, `FUN_0068BDC0 @ 0x0068BDC0`, `ScenarioClass__Create_Houses @ 0x00687F10`, `ScenarioClass__AssignStartingPoints @ 0x005EE9D0`, `ScenarioClass__Gather_Start_Positions @ 0x00688380`.
- Ghidra read memory: Battle vtable `0x007EE184`, `+0x80` entry `0x005D6BE0`, `+0x84` entry `0x005D6C70`.
- Ghidra assembly context: `0x005D6BE0`, `0x005D6C70`, especially `0x005D6BEC`, `0x005D6C12`, `0x005D6C2F`.
- Prior context docs checked: `SKIRMISH_START_GAME_HANDOFF_SESSION_PACKING_GHIDRA_REPORT.md`, `SKIRMISH_START_SESSION_VTABLE_0X14_ACCEPTANCE_GHIDRA_REPORT.md`, `SKIRMISH_START_POSITION_UX_GHIDRA_REPORT.md`, `SPAWN_POINT_ASSIGNMENT_SYSTEM.md`.
- Rust status scan: `src/ui/skirmish_shell/state.rs`, `src/ui/main_menu.rs`, `src/app.rs`, `src/app_skirmish.rs`, `src/sim/game_options.rs`.
