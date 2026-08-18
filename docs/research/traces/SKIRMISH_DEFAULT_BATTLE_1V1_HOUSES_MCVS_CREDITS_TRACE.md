# Skirmish Default Battle 1v1 Houses / MCVs / Credits Trace

Scenario: standard offline Skirmish Battle, one human America vs one Easy Russia AI, normal multiplayer map with at least two start waypoints, default credits 10000, Bases=yes, UnitCount=10.

Scope: Start Game through first in-game frame. This trace covers created active houses, local/AI owner identity, opening MCV types/positions/facing, and starting credits. It does not generalize to Team Game, Unholy, Siege, random maps, custom modes, or maps with fewer than two valid start waypoints.

## Evidence Sources

- Current Rust source: `src/ui/skirmish_shell/state.rs`, `src/skirmish_launch.rs`, `src/app.rs`, `src/app_init.rs`, `src/app_skirmish.rs`, `src/sim/world/world_spawn.rs`, `src/sim/house_state.rs`, `src/sim/game_options.rs`.
- INI data: `ini/rulesmd.ini`.
- Existing verified reports: `skirmish-ui/SKIRMISH_START_GAME_TO_SPAWN_CONSUMERS_GHIDRA_REPORT.md`, `skirmish-ui/SKIRMISH_POST_SHELL_START_UNIT_BUDGET_GHIDRA_REPORT.md`, `skirmish-ui/SKIRMISH_SPAWN_PLACEMENT_AFTER_ASSIGNED_START_GHIDRA_REPORT.md`, `skirmish-ui/SKIRMISH_MCV_NEARBY_PLACEMENT_FALLBACK_00688ED0_GHIDRA_REPORT.md`, `skirmish-ui/SKIRMISH_PACKED_OPTION_GLOBAL_CONSUMERS_GHIDRA_REPORT.md`.
- Live read-only Ghidra spot checks this run: `ScenarioClass__Create_Houses @ 0x00687F10`, `ScenarioClass__Post_Map_Init @ 0x00686890`, `FUN_005D6D80 @ 0x005D6D80`, `FUN_005D7030 @ 0x005D7030`.

All gamemd references used below are active in standard YR selected offline Skirmish unless marked conditional. The active branch is selected MPModes Battle, not the null-mode `Generate_Random_Units` path.

## Pipeline

Start Game `0x617`
-> session/node/AI/options packing
-> `ScenarioClass::Full_Init`
-> `ScenarioClass::Create_Houses`
-> Battle start preassignment / `AssignStartingPoints`
-> `ScenarioClass::Post_Map_Init`
-> selected mode `+0x84`
-> `FUN_005D6D80`
-> standard Battle `+0xC8` MCV callback `0x005D7030`
-> standard Battle `+0xCC` extra-unit callback
-> first playable frame.

Current Rust:

native shell `StartGame`
-> `SkirmishLaunchSession`
-> `App::start_skirmish_session`
-> `initialize_from_config(..., skirmish_launch_session)`
-> `apply_skirmish_launch_session`
-> direct `Simulation::spawn_object` for each assigned slot
-> first playable frame.

## Stage Trace

### Stage 1 - Start Game session data

Rust output:

- Local slot country: `LaunchCountry::America`, side index `0`, start `Auto`, team `None`.
- AI slot 1 country: `LaunchCountry::Russia`, side index `1`, difficulty `Easy` as integer `0`, start `Auto`, team `None`.
- Options from defaults: `starting_credits=10000`, `unit_count=10`, `bases=true`, `game_speed=1`.

Rust evidence: `src/skirmish_launch.rs:71..85`, `src/skirmish_launch.rs:117..124`, `src/skirmish_launch.rs:154..202`, `src/ui/skirmish_shell/state.rs:1029..1045`, `src/ui/skirmish_shell/state.rs:1958..2060`.

gamemd output:

- Start branch writes local node records and AI arrays, with credits in `DAT_00A8B25C`, unit count in `DAT_00A8B270`, AI difficulty in the AI array, and country/color/start/team fields consumed by `Create_Houses`.
- `rulesmd.ini` defaults: `Money=10000`, `UnitCount=10`, `Bases=yes`, `GameSpeed=1`, `AIDifficulty=0`.

gamemd evidence: `SKIRMISH_START_GAME_TO_SPAWN_CONSUMERS_GHIDRA_REPORT.md:18..29`, `rulesmd.ini:3017..3042`.

Verdict: PASS for this concrete local/AI/options payload.

### Stage 2 - House creation and local/AI ownership

Rust output:

- `apply_skirmish_launch_session` clears houses/AI players, populates non-player houses, then adds launch houses.
- Local active house: owner name from `session.player_name`, country `"Americans"`, `is_human=true`, side index `0`, credits `10000`.
- AI active house: owner name `"Computer1"`, country `"Russians"`, `is_human=false`, side index `1`, credits `10000`; one `AiPlayerState` is pushed.

Rust evidence: `src/app_skirmish.rs:181..186`, `src/app_skirmish.rs:253..317`, `src/sim/house_state.rs:52..76`.

gamemd output:

- `ScenarioClass__Create_Houses @ 0x00687F10` creates human houses from node records, AI houses from AI arrays, sets `g_PlayerPtr` on the local human, and calls `HouseClass__Set_Credits_And_Color(..., DAT_00A8B25C)` for both human and AI houses.
- Human house country index comes from node `+0x4B`; AI house country index from `DAT_00A8B29C`; AI difficulty is passed into `HouseClass__SetDifficulty`.

gamemd evidence: live Ghidra decompile `0x00687F10`; `SKIRMISH_START_GAME_TO_SPAWN_CONSUMERS_GHIDRA_REPORT.md:39..43`, `SKIRMISH_PACKED_OPTION_GLOBAL_CONSUMERS_GHIDRA_REPORT.md:107`.

Verdict: PASS for active local/AI country, control role, Easy difficulty value `0`, and initial credit value `10000`. Total internal house roster size is UNCHECKED because this trace did not enumerate the selected map's full non-player house list in both engines.

### Stage 3 - Start assignment

Rust output:

- `assign_launch_starts` reads multiplayer starts, applies explicit starts first, then assigns `Auto` slots to the first unused waypoint in map order.
- With two Auto slots and at least two starts, local gets `starts[0]`; AI gets `starts[1]`.

Rust evidence: `src/app_skirmish.rs:188..190`, `src/app_skirmish.rs:375..438`.

gamemd output:

- Battle `+0x80` copies explicit start choices from `House+0x16058` to `ScenarioClass+0x1180`.
- `AssignStartingPoints @ 0x005EE9D0` gathers start positions, handles human houses first and AI second, and uses `FUN_005EE6F0` policy for unassigned starts.
- For unoccupied starts, native selection is not simply "first unused waypoint"; it may choose random or distance-based starts depending on occupied-count state.

gamemd evidence: `SKIRMISH_START_GAME_TO_SPAWN_CONSUMERS_GHIDRA_REPORT.md:43..45`, `SKIRMISH_SPAWN_PLACEMENT_AFTER_ASSIGNED_START_GHIDRA_REPORT.md:40..56`.

Verdict: UNCHECKED for exact local/AI waypoint equality. The scenario did not name a map or a native RNG transcript, so literal start-cell equality cannot be computed.

### Stage 4 - Opening MCV type

Rust output:

- America: `LaunchCountry::America.opening_mcv_candidates()` yields `["AMCV","SMCV","PCV"]`; `launch_mcv_type_for_country` picks `AMCV` because it exists in rules.
- Russia: candidates `["SMCV","AMCV","PCV"]`; Rust picks `SMCV`.

Rust evidence: `src/skirmish_launch.rs:79..85`, `src/app_skirmish.rs:193..207`, `src/app_skirmish.rs:836..843`.

gamemd output:

- Standard Battle `+0xC8` callback `0x005D7030` resolves `[General] BaseUnit` via `FUN_00505310(Rules+0xB20)`.
- Stock YR `[General] BaseUnit=AMCV,SMCV,PCV`.
- `AMCV Owner=British,French,Germans,Americans,Alliance`; `SMCV Owner=Russians,Confederation,Africans,Arabs`.

gamemd/INI evidence: live Ghidra decompile `0x005D7030`; `SKIRMISH_POST_SHELL_START_UNIT_BUDGET_GHIDRA_REPORT.md:103..114`, `rulesmd.ini:390`, `rulesmd.ini:6969..6983`, `rulesmd.ini:7838..7851`.

Verdict: PASS for this concrete America/Russia MCV type output: local `AMCV`, AI `SMCV`.

### Stage 5 - Opening MCV position and blocked-start fallback

Rust output:

- Rust calls `Simulation::spawn_object(mcv_type, owner, waypoint.rx, waypoint.ry, 64, rules, height_map)`.
- Entity position is exactly the assigned waypoint cell with center subcell leptons; no startup placement probe or nearby fallback is attempted.
- If spawn fails, Rust logs and may clear `local_owner` for the human slot.

Rust evidence: `src/app_skirmish.rs:193..235`, `src/sim/world/world_spawn.rs:277..335`, `src/sim/game_entity.rs:314..337`.

gamemd output:

- Standard selected Battle startup writes assigned base cell to `House+0x5490`.
- `0x005D7030` creates the MCV, converts `House+0x5494` if valid else `House+0x5490` to centered coordinates, then calls object `Place`.
- If exact placement fails, it calls `FUN_00688ED0(mcv, base_cell, 1)`, which expands through radius `1..31` with random-start clockwise directions plus a jitter pass before deletion.

gamemd evidence: live Ghidra decompile `0x005D7030`; `SKIRMISH_SPAWN_PLACEMENT_AFTER_ASSIGNED_START_GHIDRA_REPORT.md:64..80`, `SKIRMISH_MCV_NEARBY_PLACEMENT_FALLBACK_00688ED0_GHIDRA_REPORT.md:22..24`, `SKIRMISH_MCV_NEARBY_PLACEMENT_FALLBACK_00688ED0_GHIDRA_REPORT.md:64..103`.

Verdict: NOT-IMPLEMENTED for native placement/fallback behavior. Exact final MCV cell for an unblocked named map is UNCHECKED because no map and native RNG transcript were specified.

### Stage 6 - Opening MCV facing

Rust output:

- Rust passes facing `64` to `spawn_object`; `GameEntity.facing=64`.
- In Rust comments this is east in RA2 byte-facing convention.

Rust evidence: `src/app_skirmish.rs:198..205`, `src/sim/world/world_spawn.rs:278..289`, `src/sim/game_entity.rs:76..77`.

gamemd output:

- The checked standard callback constructs `UnitClass` and places it, but this trace did not resolve the exact constructor/default facing or any placement-time facing write for startup MCVs.

gamemd evidence: live Ghidra decompile `0x005D7030` shows no explicit facing immediate; deeper `UnitClass__Constructor`/`Unlimbo` facing state was not decoded in this slot.

Verdict: UNCHECKED. Rust value is `64`; gamemd value was not computed.

### Stage 7 - Starting credits at first frame

Rust output:

- Both active launch houses are created with `sim.game_options.starting_credits`, which is `10000` from the launch session/defaults.
- No later first-frame credit bonus from the selected-mode start generator is implemented.

Rust evidence: `src/app_skirmish.rs:177..180`, `src/app_skirmish.rs:298..317`, `src/sim/game_options.rs:55..78`.

gamemd output:

- `Create_Houses` calls `HouseClass__Set_Credits_And_Color(..., DAT_00A8B25C)`, and default `DAT_00A8B25C` is `10000`.
- `FUN_005D6D80` can optionally add leftover credits after extra-unit generation. For this concrete default `UnitCount=10`, the exact leftover amount was not computed because the full `+0xCC` allocation transcript was outside this run.

gamemd evidence: live Ghidra decompile `0x00687F10`, live Ghidra decompile `0x005D6D80`, `SKIRMISH_PACKED_OPTION_GLOBAL_CONSUMERS_GHIDRA_REPORT.md:107`, `SKIRMISH_POST_SHELL_START_UNIT_BUDGET_GHIDRA_REPORT.md:78..95`.

Verdict: PASS for initial house creation balance `10000`. UNCHECKED for final first-frame balance after native extra-unit generation/leftover-credit handling.

### Stage 8 - Default UnitCount first-frame contents

Rust output:

- Rust stores `unit_count=10` in `sim.game_options`, but `apply_skirmish_launch_session` only spawns one MCV per assigned active slot.
- No native budgeted extra starting unit generation was found in this path.

Rust evidence: `src/skirmish_launch.rs:180..202`, `src/app_skirmish.rs:162..248`.

gamemd output:

- `Post_Map_Init` calls selected mode `+0x84`, then `FUN_005D6D80`.
- With `DAT_00A8B270=10`, `FUN_005D6D80` computes a money-like budget from eligible unit/infantry costs, calls mode `+0xC8` for MCV/base unit, then mode `+0xCC` for extra units per non-special/non-observer house.

gamemd evidence: live Ghidra decompile `0x00686890`, live Ghidra decompile `0x005D6D80`, `SKIRMISH_POST_SHELL_START_UNIT_BUDGET_GHIDRA_REPORT.md:22..26`, `SKIRMISH_POST_SHELL_START_UNIT_BUDGET_GHIDRA_REPORT.md:78..95`.

Verdict: NOT-IMPLEMENTED. On default UnitCount, gamemd first frame has startup-unit generation beyond the MCV path; Rust does not.

## Entry Points Covered

1. Native shell launch-session path: `state.rs` builds `SkirmishLaunchSession`; `app.rs` stores it; `app_init.rs` passes it to `apply_skirmish_launch_session`.
2. Legacy/simple skirmish path: `start_selected_skirmish` clears `pending_skirmish_launch_session` and falls back to `seed_skirmish_opening_if_needed`.

This trace's concrete scenario is the native shell launch-session path. The legacy/simple path is not the main scenario, but it remains a risk because it bypasses per-row launch data.

## Top Findings

1. Stage 8 / NOT-IMPLEMENTED: default `UnitCount=10` startup units are missing; Rust stores the option but spawns only MCVs. Rust: `src/app_skirmish.rs:162..248`; gamemd: `Post_Map_Init -> FUN_005D6D80 -> +0xCC`.
2. Stage 5 / NOT-IMPLEMENTED: native startup MCV placement/fallback is missing; blocked start cells fail/log in Rust instead of `FUN_00688ED0` radius `1..31` fallback. Rust: `src/app_skirmish.rs:193..235`, `src/sim/world/world_spawn.rs:277..335`; gamemd: `0x005D7030`, `0x00688ED0`.
3. Stage 3 / UNCHECKED: exact local/AI waypoint equality is not proven; Rust uses first-unused order, while gamemd's unassigned-start picker uses occupied-table/random/distance policy. Rust: `src/app_skirmish.rs:375..438`; gamemd: `0x005EE9D0`, `FUN_005EE6F0`.
4. Stage 6 / UNCHECKED: Rust hardcodes startup MCV facing `64`; gamemd startup MCV facing was not computed from constructor/placement state. Rust: `src/app_skirmish.rs:198..205`; gamemd evidence incomplete.
5. Stage 7 / UNCHECKED: house creation balance matches `10000`, but first-frame balance after native extra-unit leftover-credit handling was not computed. Rust: `src/app_skirmish.rs:298..317`; gamemd: `FUN_005D6D80`.

## Verdict Tally

PASS: 4
FAIL: 0
UNCHECKED: 4
NOT-IMPLEMENTED: 2

COMPLETE for this slot's bounded source/doc/Ghidra trace. Partial only for exact named-map waypoint cells, gamemd startup MCV facing, and default UnitCount allocation transcript, which require a named map plus runtime/binary value capture.
