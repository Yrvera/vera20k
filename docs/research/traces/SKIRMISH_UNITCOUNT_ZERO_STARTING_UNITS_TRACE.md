# Skirmish UnitCount=0 Starting Units Trace

Trace target: standard offline Skirmish Battle, one human and one AI, Unit Count trackbar set to `0`, default credits and other options. Scope is Start Game option packing through post-map initialization and first in-game frame, only for whether `UnitCount=0` grants extra starting units beyond opening MCVs.

## Summary

YR result for this scenario: `UnitCount=0` produces extra-unit budget `0`, so the standard extra-unit callback returns before creating any non-MCV starting units. It does not suppress opening MCV creation; default `Bases=yes` still allows the selected mode's MCV/base callback.

Current Rust result: the native launch-session path stores `unit_count=0` and copies it into `Simulation.game_options.unit_count`, then spawns one MCV per assigned launch slot. No Rust post-map start-unit budget consumer exists, so it also creates `0` extra units for this scenario, but not through equivalent logic.

## Pipeline

`Shell Unit Count=0` -> `SkirmishLaunchSession.options.unit_count=0` -> `Simulation.game_options.unit_count=0` -> `apply_skirmish_launch_session` creates launch houses and MCVs -> no Rust UnitCount budget/spending stage -> first frame contains opening MCVs only if placement succeeded.

YR pipeline: `0x50C trackbar` -> `DAT_00A8B270=0` -> `ScenarioClass::Post_Map_Init @ 0x00686890` -> selected Battle mode `+0x84` -> `FUN_005D6D80` -> standard `+0xC8 @ 0x005D7030` MCV callback -> standard `+0xCC @ 0x005D70F0` returns immediately because `*budget <= 0`.

## Stage Trace

### Stage 1 - Unit Count UI Range

- Input: player sets Unit Count to `0`.
- Rust: `UNIT_COUNT_MIN=0`, `UNIT_COUNT_MAX=10`, `UNIT_COUNT_STEP=1`; visual value writes `state.unit_count = visual_value` at `src/ui/skirmish_shell/state/trackbars.rs:23` and `:120`.
- gamemd: `[MultiplayerDialogSettings] MinUnitCount=0`, `UnitCount=10`, `MaxUnitCount=10` in `ini/rulesmd.ini:3022..3024`; shell report maps control `0x50C` to `DAT_00A8B270`.
- Output equality: Rust accepts/stores `0`; YR accepts/stores `0`.
- Verdict: PASS.

### Stage 2 - Start Game Option Packing

- Input: shell state `unit_count=0`.
- Rust: `launch_session` writes `options.unit_count = state.unit_count` at `src/ui/skirmish_shell/state/launch.rs:171..174`; `SkirmishLaunchOptions.unit_count` exists at `src/skirmish_launch.rs:134..137`.
- gamemd: Start handoff reads trackbar `0x50C` with message `0x400`, stores `DAT_00A8B270`, and mirrors to `DAT_00A8B3D4`; active in YR per `SKIRMISH_START_GAME_HANDOFF_SESSION_PACKING_GHIDRA_REPORT.md:110..112`.
- Output equality: packed UnitCount value is `0` on both sides.
- Verdict: PASS.

### Stage 3 - Session To Simulation Options

- Input: `SkirmishLaunchSession.options.unit_count=0`.
- Rust: `apply_skirmish_launch_session` calls `to_game_options` at `src/app_skirmish.rs:177..179`; `to_game_options` copies `unit_count: self.unit_count` at `src/skirmish_launch.rs:179..197`; `GameOptions.unit_count` exists at `src/sim/game_options.rs:40..44`.
- gamemd: `DAT_00A8B270` remains the post-map UnitCount global consumed by `FUN_005D6D80`; active in YR per `SKIRMISH_POST_SHELL_START_UNIT_BUDGET_GHIDRA_REPORT.md:30..35`.
- Output equality: stored match option value is `0` on both sides.
- Verdict: PASS.

### Stage 4 - Post-Map Entry

- Input: selected standard Battle mode, map loaded, houses initialized.
- Rust: `app_init.rs:518..527` calls `apply_skirmish_launch_session` before first in-game frame when a launch session exists.
- gamemd: read-only Ghidra decompile of `ScenarioClass__Post_Map_Init @ 0x00686890` confirms standard non-map-editor flow: if selected mode object `DAT_00A8B23C` is non-null, call vtable `+0x84`, then `FUN_005D6D80`; active because this is the selected-mode offline Skirmish path.
- Output equality: exact call timing relative to first rendered frame was not numerically measured in Rust and gamemd.
- Verdict: UNCHECKED.

### Stage 5 - UnitCount Budget Computation

- Input: `UnitCount=0`.
- Rust: no post-map start-unit budget stage reads `Simulation.game_options.unit_count`; `rg unit_count` shows storage/hash/test surfaces but no spawn consumer.
- gamemd: read-only Ghidra decompile of `FUN_005D6D80 @ 0x005D6D80` confirms the budget-building block runs only when `0 < DAT_00A8B270`; with `DAT_00A8B270=0`, the budget pointer/value remains zero and house iteration continues. Existing report lines `76..88` give the positive-count formula: `(((eligible_count / 2 + total_cost) / eligible_count) * UnitCount)`.
- Output equality: YR computed extra-unit budget is exactly `0`; Rust has no computed budget value.
- Verdict: NOT-IMPLEMENTED.

### Stage 6 - Opening MCV Callback

- Input: default `Bases=yes`, one human plus one AI.
- Rust: `apply_skirmish_launch_session` assigns starts and spawns `launch_mcv_type_for_country(...)` once per assigned slot at `src/app_skirmish.rs:188..210`.
- gamemd: read-only Ghidra decompile of standard `+0xC8 @ 0x005D7030` confirms it returns immediately only when `DAT_00A8B258 == 0`; otherwise it selects a BaseUnit from `Rules+0xB20`, constructs a UnitClass, tries direct placement, then fallback `FUN_00688ED0`.
- Output equality: likely two opening MCVs on a normal valid 1v1 Battle map, but exact map, start cells, MCV type masks, placement result, and positions were not computed for both engines.
- Verdict: UNCHECKED.

### Stage 7 - Extra Starting Units

- Input: extra-unit budget `0`.
- Rust: no extra-unit generator exists, so current Rust spawns `0` non-MCV starting units from UnitCount.
- gamemd: `SKIRMISH_POST_SHELL_START_UNIT_BUDGET_GHIDRA_REPORT.md:116..129` verifies standard `+0xCC @ 0x005D70F0`; it reads the `int*` budget and returns true immediately if `*budget <= 0`. For `UnitCount=0`, `*budget=0`, so extra units spawned: `0`.
- Output equality: non-MCV starting units from UnitCount are `0` in Rust and `0` in YR.
- Verdict: PASS.

### Stage 8 - First In-Game Frame Observable Result

- Expected YR: opening MCVs only, no extra infantry/vehicles from the UnitCount budget.
- Expected Rust: opening MCVs only on the launch-session path, no extra infantry/vehicles from UnitCount.
- Output equality: extra-unit count equality is proven (`0 == 0`); full entity list/positions/types are not proven for the whole first frame.
- Verdict: PASS for UnitCount extra-unit count; UNCHECKED for full frame parity.

## Findings

1. NOT-IMPLEMENTED - Stage 5 - Rust stores `unit_count` but has no native post-map average-cost budget consumer. This is not player-visible at `UnitCount=0` for extra-unit count, but becomes visible for positive UnitCount. Rust: `src/app_skirmish.rs:162..248`; gamemd: `FUN_005D6D80 @ 0x005D6D80`.
2. UNCHECKED - Stage 6 - MCV type and placement are not numerically proven equal for the selected map. Rust uses `launch_mcv_type_for_country` and direct `spawn_object`; gamemd uses `[General] BaseUnit`, side masks, direct Place, and `FUN_00688ED0` fallback.

## Verdict Tally

PASS: 4 | FAIL: 0 | UNCHECKED: 3 | NOT-IMPLEMENTED: 1

## Status

COMPLETE
