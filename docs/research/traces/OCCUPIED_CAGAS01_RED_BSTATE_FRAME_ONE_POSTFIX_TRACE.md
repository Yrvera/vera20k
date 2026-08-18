# Occupied CAGAS01 Red BState Frame-One Postfix Trace

## Scope

Single concrete scenario only: stock Yuri's Revenge `CAGAS01`, occupied, `TechLevel=-1`, health at the red threshold, and native `BuildingClass+0x534`/BState nonzero so `BuildingClass::GetCurrentFrame` enters the `CanBeOccupied` body-frame formula.

Concrete numeric sample used for equality checks: `occupant_count=1`, `health_current=250`, `health_max=1000`, `ConditionRed=25%`, `ConditionYellow=50%`, `TechLevel=-1`.

## Sources Read

- `ini/rulesmd.ini:752-753`: `ConditionRed=25%`, `ConditionYellow=50%`.
- `ini/rulesmd.ini:19302-19325`: `CAGAS01`, `TechLevel=-1`, `Strength=1000`, `CanBeOccupied=yes`, `CanOccupyFire=yes`.
- `ini/artmd.ini:8019-8041`: `CAGAS01` has static art, no live active/idle occupied slot relevant to this body-frame trace.
- `docs/research/GARRISON_OCCUPIED_BUILDING_VISUAL_STATE_GHIDRA_REPORT.md:42-70`: verified active YR `GetCurrentFrame` BState gate and occupied civilian red collapse.
- `src/app_instances/shp.rs:141-160`, `src/app_instances/shp.rs:745-808`, `src/app_instances/shp.rs:1006-1010`: current post-fix Rust render path and focused regression test.

## Pipeline

Trigger to screen:

`occupied CAGAS01 at red health` -> render sees `CanBeOccupied=yes` -> counts occupants -> reads health and rules thresholds -> BState/damage gate admits body-frame formula -> occupied base frame `2` -> red tier increments to `3` -> civilian collapse maps `3` to `1` -> body SHP frame `1` is selected.

## Stage Results

1. Stock data: PASS

   Gamemd/Rust data for this scenario is the same from stock INI: `CAGAS01 TechLevel=-1`, `Strength=1000`, `CanBeOccupied=yes`, `ConditionRed=0.25`, `ConditionYellow=0.5`. Rust parses percent values with `%` divided by 100 in `src/rules/ini_parser.rs:100-106` and stores them in `src/rules/ruleset.rs:826-905`.

2. Native active-YR formula: PASS

   Verified docs say active YR `GetCurrentFrame` reads `BuildingClass+0x534`, enters the `CanBeOccupied` formula only when that value is nonzero, starts occupied buildings at frame `2`, increments to `3` at red health, then collapses `TechLevel == -1 && frame == 3` to `1`.

   Numeric gamemd result for the concrete sample: BState nonzero, occupants positive -> `2`; `250 / 1000 = 0.25`; `0.25 <= ConditionRed 0.25` -> `3`; `TechLevel=-1 && frame=3` -> `1`.

3. Current Rust final body-frame computation: PASS

   Current Rust at `src/app_instances/shp.rs:153-160` passes `occupant_count`, `health_current`, `health_max`, `tech_level`, `ConditionYellow`, and `ConditionRed` into `rendered_garrison_body_frame_index`.

   Numeric Rust result for the same sample: health gate `250 / 1000 = 0.25`; `0.25 <= ConditionYellow 0.5` -> gate true; occupants positive -> `base=2`; red check `0.25 <= 0.25` -> true; yellow check is false because `TechLevel=-1` is not `> 0`; increment -> `3`; civilian collapse -> `1`.

   Final equality: gamemd body frame `1`, Rust body frame `1`.

4. Focused test coverage: PASS

   Current Rust contains `occupied_civilian_garrison_bstate_red_collapses_to_frame_one` at `src/app_instances/shp.rs:1006-1010`, asserting `rendered_garrison_body_frame_index(1, 20, 100, -1, 0.5, 0.25) == 1`.

5. Native BState byte modeling: NOT-IMPLEMENTED

   Rust does not currently carry an exact modeled equivalent of native `BuildingClass+0x534`; `src/app_instances/shp.rs:753-774` uses `health_current / health_max <= ConditionYellow` as a render-side proxy for "BState/damage active." This does not change the final frame for the concrete red-health, BState-active scenario traced here, but it is not a byte-equivalent model of the native field and remains a broader parity gap.

## Verdict

For the exact requested scenario, current Rust computes rendered body SHP frame `1`, matching gamemd's active-YR `GetCurrentFrame` output.

Verdict tally: PASS: 4 | FAIL: 0 | UNCHECKED: 0 | NOT-IMPLEMENTED: 1

Status: COMPLETE

## Adjacent Findings

- Native BState writer timing and exact stored values are outside this trace. Existing docs verify the render-side gate, but Rust still needs a true building visual/BState field to remove the health-derived proxy.
