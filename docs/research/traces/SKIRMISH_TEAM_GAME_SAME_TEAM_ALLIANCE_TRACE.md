# Skirmish Team Game Same-Team Alliance Trace

**Scenario:** Offline Skirmish Team Game-style launch with local human on Team A, AI1 on Team A, and AI2 on Team B so Start validation can pass.

**Scope:** Start Game packing through house creation and launch-time alliance state before normal play. No Rust edits. No INI edits.

## Verdict

PASS: 5 | FAIL: 0 | UNCHECKED: 1 | NOT-IMPLEMENTED: 0

Top FAIL / NOT-IMPLEMENTED findings: none in the scoped native-shell scenario.

Status: COMPLETE

## Pipeline

Team Game mode selection -> Team combo values -> Start Game validation -> launch-session packing -> house/slot creation -> same-team alliance map -> normal play consumes friendly/enemy relations.

## Entry Points

1. Native skirmish shell Start Game: `src/ui/skirmish_shell/state/launch.rs:95` packages `SkirmishLaunchSession`, `src/app.rs:481` stores it as pending launch state, and `src/app_init.rs:519` applies it during map load. This is the scoped path.
2. Legacy egui fallback Start: `src/app.rs:461` clears `pending_skirmish_launch_session` and falls back to simplified skirmish seeding. It does not represent this Team Game same-team handoff scenario and is not scored here.

## Stage 1 - Team Game Team Values

Input: selected Team Game mode, local Team A, AI1 Team A, AI2 Team B.

Rust: `MPTeamMD.ini` maps to `must_ally=true` in `src/skirmish_modes.rs:73`; `combo_items` emits Team values `[0, 1, 2, 3]` when `selected_mode_must_ally` is true at `src/ui/skirmish_shell/state.rs:507`. `repair_teams_for_selected_mode` repairs `-2` to `0` for the player and to `3` for active AIs in Team Game at `src/ui/skirmish_shell/state/player_name.rs:336`.

gamemd: Team Game suppresses Team `None`; the selected-mode `+0x2C` callback returns no negative `None` row when `MustAlly` is true, then appends explicit teams `0..3`. Active in standard YR offline Skirmish per `SKIRMISH_TEAM_ADJUNCT_HOUSE_ALLIANCE_HANDOFF_GHIDRA_REPORT.md:41` and `:97`.

Output equality: Rust Team rows `[0,1,2,3]`; gamemd Team rows `[0,1,2,3]`.

Verdict: PASS.

## Stage 2 - Start Validation

Input: local explicit team `0`, active AI teams `[0, 1]`, active AI count `2`, requested players `3`.

Rust: `launch_session` rejects same-explicit-team only if `state.player_team >= 0` and every active AI maps to the same `LaunchTeam::Team(local_team)` at `src/ui/skirmish_shell/state/launch.rs:117`. For `[0,1]`, `all_active_ai_same_team=false`, so no `SameExplicitTeam` error.

gamemd: Start validation skips negative local teams and blocks only when the local team is explicit and every active AI has the same explicit team. Active in YR; evidence `0x006ACEE0`, branch `0x006AD16C..0x006AD236`, documented at `SKIRMISH_TEAM_ADJUNCT_HOUSE_ALLIANCE_HANDOFF_GHIDRA_REPORT.md:65`.

Output equality: Rust decision `allow`; gamemd decision `allow`.

Verdict: PASS.

## Stage 3 - Start Game Packing

Input: local shell team `0`, AI1 shell team `0`, AI2 shell team `1`.

Rust: `LaunchTeam::from_shell_value` maps non-negative values to `Team(u8)` at `src/skirmish_launch.rs:101`. `launch_session` writes local team at `src/ui/skirmish_shell/state/launch.rs:139` and opponent teams at `src/ui/skirmish_shell/state/launch.rs:157`.

Computed Rust packed values: local `Team(0)`, AI1 `Team(0)`, AI2 `Team(1)`.

gamemd: Start Game writes local Team to node `+0x63`, AI Team to `DAT_00A8B2FC[slot]`, with values `0`, `0`, `1` for this scenario. Active in YR; evidence `0x006ACEE0` and bytes `0x006AD4C7..0x006AD4E6`, documented at `SKIRMISH_TEAM_ADJUNCT_HOUSE_ALLIANCE_HANDOFF_GHIDRA_REPORT.md:63`.

Output equality: team payload `[0,0,1]` in both.

Verdict: PASS.

## Stage 4 - House / Slot Creation Boundary

Input: launch team payload `[0,0,1]`.

Rust: `apply_skirmish_launch_session` normalizes slots at `src/app_skirmish.rs:171`, then creates local owner plus `Computer1` and `Computer2` at `src/app_skirmish.rs:253`.

gamemd: `ScenarioClass__Create_Houses @ 0x00687F10` copies Team into `House+0x1605C`, producing `[0,0,1]` for the three playable houses. Active in YR and non-campaign Skirmish; documented at `SKIRMISH_TEAM_ADJUNCT_HOUSE_ALLIANCE_HANDOFF_GHIDRA_REPORT.md:69`.

Output equality: alliance-consumer team values `[0,0,1]` in both. Rust does not store a `House+0x1605C` field; the equivalent consumer input is the normalized slot team.

Verdict: PASS.

## Stage 5 - Same-Team Alliance Handoff

Input: playable slots `Player:0`, `Computer1:0`, `Computer2:1`.

Rust: `launch_alliance_map` starts from map alliances, inserts all slot keys, skips non-explicit teams, and inserts both directions for equal explicit teams at `src/app_skirmish.rs:324`. Computed scoped graph: `PLAYER -> COMPUTER1`, `COMPUTER1 -> PLAYER`, no `PLAYER -> COMPUTER2`, no `COMPUTER1 -> COMPUTER2`.

gamemd: `ScenarioClass__Post_Map_Init @ 0x00686990` calls selected mode vtable `+0x88`; direct read-only Ghidra decompile confirms the `+0x88` call in the active post-map path. Battle-style vtable binds `+0x88` to `0x005D74A0`, which skips `-2/-1`, compares `House+0x1605C`, and calls `HouseClass__MakeAlly @ 0x004F9B70` twice for equal explicit pairs. Direct read-only Ghidra decompile of `HouseClass__MakeAlly` confirms it sets `this->Allies |= 1 << otherHouse->ArrayIndex`. Existing report evidence: `SKIRMISH_TEAM_ADJUNCT_HOUSE_ALLIANCE_HANDOFF_GHIDRA_REPORT.md:81` and `:88`.

Output equality: Player and AI1 are mutual allies; AI2 is not allied with either Team A house.

Verdict: PASS.

## Stage 6 - Timing / Visibility

Rust: alliance graph is installed before MCV spawning in `apply_skirmish_launch_session` at `src/app_skirmish.rs:185`.

gamemd: same-team alliances are applied from `ScenarioClass__Post_Map_Init` after house creation/start setup/base-unit setup and before normal play, via selected-mode `+0x88`.

Player-visible result at first normal play frame: same Team A slots are friendly and Team B remains enemy.

Exact loading-frame/EVA notification timing was not computed for both engines. The existing Ghidra report explicitly leaves `MakeAlly` notification/EVA timing out of scope.

Verdict: UNCHECKED.

## Notes

No tests were run, because preserving the "write exactly one file" constraint means avoiding build/test commands that may update `target/`. Existing focused tests are present at `src/app_skirmish.rs:753`, `src/ui/skirmish_shell/state/tests.rs:1113`, and `src/ui/skirmish_shell/state/tests.rs:1169`.
