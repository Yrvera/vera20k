# Occupied CAGAS01 Yellow BState Frame Two - Postfix Trace

Status: COMPLETE  
Scenario: stock Yuri's Revenge `CAGAS01`, occupied, `TechLevel=-1`, health ratio `<= ConditionYellow` and `> ConditionRed`, with native `BuildingClass+0x534`/BState already nonzero so `BuildingClass::GetCurrentFrame` enters the `CanBeOccupied` body-frame formula.

## Verdict

PASS: 4 | FAIL: 0 | UNCHECKED: 2 | NOT-IMPLEMENTED: 0

The current Rust post-fix render helper computes rendered body SHP frame `2` for the concrete yellow-health BState-active occupied civilian case. Full native BState writer/timing equivalence remains unchecked because Rust currently derives the render gate from health ratio rather than storing a byte-equivalent `BuildingClass+0x534` state.

## Pipeline

`rulesmd.ini` CAGAS01 data -> occupied cargo count -> yellow/red threshold data -> BState/damage gate -> garrison body-frame formula -> body SHP frame index

## Evidence Inputs

- Stock `CAGAS01` is active YR data with `TechLevel=-1`, `Strength=1000`, and `CanBeOccupied=yes`: `ini/rulesmd.ini:19302..19322`.
- Stock thresholds are `ConditionYellow=50%` and `ConditionRed=25%`: `ini/rules.ini:610..611`.
- Rust parses those percentages into `0.5` and `0.25`: `src/rules/ini_parser.rs:100..106`, `src/rules/ruleset.rs:826..831`.
- Concrete yellow sample used for literal computation: `health_current=400`, `health_max=1000`, ratio `0.4`; this is `<= 0.5` and `> 0.25`.
- Concrete occupant count: `1`; Rust cargo count is `passengers.len() as u32`: `src/sim/passenger.rs:58..60`.

## gamemd Computation

Verified active YR function: `BuildingClass::GetCurrentFrame @ 0x0043EF90`.

For this scenario:

1. `BuildingClass+0x534 != 0`, so the BState gate enters the damaged/body formula path.
2. `Type+0x157B CanBeOccupied=yes`, so the occupied-building formula is active.
3. Occupant count is positive, so base frame becomes `2`.
4. Ratio `0.4 <= ConditionRed 0.25` is false, so red does not increment.
5. Civilian `TechLevel=-1` means the yellow increment is skipped; verified report states yellow occupied civilian remains frame `2`.
6. Civilian collapse `TechLevel == -1 && frame == 3 -> 1` does not apply because frame is `2`.
7. gamemd output: body SHP frame `2`.

Primary evidence:

- BState gate and active YR status: `docs/research/GARRISON_OCCUPIED_BUILDING_VISUAL_STATE_GHIDRA_REPORT.md:42..58`.
- Yellow civilian result: `docs/research/GARRISON_OCCUPIED_BUILDING_VISUAL_STATE_GHIDRA_REPORT.md:62..70`.
- Branch summary and threshold constants: `docs/research/BUILDINGCLASS_DRAWBODY_GHIDRA_REPORT.md:154..183`.
- ConditionRed is only the extra garrison health-pip threshold: `docs/research/BUILDINGCLASS_DRAWBODY_GHIDRA_REPORT.md:458..461`.

Verdict: PASS for frame index `2`.

## Current Rust Computation

Render integration:

- Structure body render checks `can_be_occupied`, reads occupant count, tech level, `ConditionYellow`, and `ConditionRed`, then calls `rendered_garrison_body_frame_index`: `src/app_instances/shp.rs:139..160`.

Concrete Rust values:

1. `can_be_occupied=true` from stock CAGAS01.
2. `occupant_count=1`.
3. `tech_level=-1`.
4. `condition_yellow=0.5`, `condition_red=0.25`.
5. `rendered_garrison_body_frame_index(1, 400, 1000, -1, 0.5, 0.25)` first checks the BState proxy: `400 / 1000 = 0.4 <= 0.5`, so it enters `building_frame_index`: `src/app_instances/shp.rs:745..764`.
6. `building_frame_index` sets `base=2` because occupant count is positive: `src/app_instances/shp.rs:790..793`.
7. `red_tier = 0.4 <= 0.25 = false`: `src/app_instances/shp.rs:799`.
8. `yellow_tier = tech_level > 0 && 0.4 <= 0.5 = false && true = false`: `src/app_instances/shp.rs:800`.
9. No increment fires; civilian red collapse does not fire because `base != 3`: `src/app_instances/shp.rs:801..807`.
10. Rust output: body SHP frame `2`.

Regression coverage exists for this exact numeric helper case: `occupied_civilian_garrison_bstate_yellow_uses_frame_two` uses `(1, 40, 100, -1, 0.5, 0.25) -> 2` at `src/app_instances/shp.rs:997..1002`.

Verdict: PASS for frame index `2`.

## Unchecked Boundaries

1. Native BState writer/timing equivalence is UNCHECKED. The verified render decision depends on `BuildingClass+0x534 != 0`; current Rust uses `health_current / health_max <= ConditionYellow` as the render gate at `src/app_instances/shp.rs:766..775`, not a stored byte-equivalent building BState. For this concrete yellow sample both booleans are true, but full state-byte parity is not proven.
2. Exact render-call timing within a full tick/frame is UNCHECKED. gamemd evidence proves `GetCurrentFrame` is the normal body render query, and Rust computes during SHP instance generation, but this trace did not run a live side-by-side frame capture.

## Adjacent Findings

- Healthy occupied `CAGAS01` without BState is a separate scenario and is not traced here.
- Red occupied `CAGAS01` collapse to frame `1` is a separate scenario and is not traced here.
- `ActiveAnimGarrisoned`/damaged active anim replacement is a separate scenario and is not traced here.
