# Occupied Civilian Yellow BState Body Frame Two Trace

## Scope

Concrete scenario only: standard Yuri's Revenge stock civilian `CanBeOccupied=yes`
building, using `CAGAS01` as the stock sample, is occupied and at yellow health
(`ConditionYellow >= health ratio > ConditionRed`) while native
`BuildingClass+0x534` BState/current building state is nonzero, so the native
damage/BState path is active.

This trace checks only the body SHP frame selected for rendering.

## Evidence

- `ini/rulesmd.ini:752-753`: `ConditionRed=25%`, `ConditionYellow=50%`.
- `ini/rulesmd.ini:19302-19323`: `CAGAS01` is a stock civilian Gas Station with
  `TechLevel=-1`, `Strength=1000`, `CanBeOccupied=yes`, and
  `MaxNumberOccupants=10`.
- `docs/research/GARRISON_OCCUPIED_BUILDING_VISUAL_STATE_GHIDRA_REPORT.md:42-70`:
  verified active-YR `BuildingClass::GetCurrentFrame @ 0x0043EF90` reads
  `BuildingClass+0x534`; when nonzero and `CanBeOccupied` is true, positive
  occupant count sets base body frame `2`; red health increments; buildable
  yellow health increments only for `TechLevel > 0`; civilian `TechLevel == -1`
  collapses occupied red frame `3` to `1`. The report explicitly states this is
  active in YR for stock examples including `CAGAS01`.
- `src/app_instances/shp.rs:138-164`: structure SHP body frame path reads
  `CanBeOccupied`, occupant count, `TechLevel`, and condition thresholds, then
  calls `rendered_garrison_body_frame_index`.
- `src/app_instances/shp.rs:670-688`: Rust renders frame `0` unless the local
  BState damage proxy is active, then calls the garrison body frame formula.
- `src/app_instances/shp.rs:691-733`: Rust BState proxy is
  `health_current / health_max <= ConditionYellow`; formula sets occupied base
  frame `2`, increments for red or buildable-yellow, and maps civilian frame
  `3` to `1`.
- `src/app_instances/shp.rs:875-879`: focused Rust test fixture asserts the
  yellow occupied civilian case returns frame `2`.

## Pipeline

1. Data: `CAGAS01` supplies `CanBeOccupied=yes`, `TechLevel=-1`, `Strength=1000`.
2. Trigger/input state: occupied count is positive and native BState is nonzero.
3. Damage tier: choose a concrete yellow sample `health_current=400`,
   `health_max=1000`; ratio is `0.4`, so `0.4 <= 0.5` and `0.4 > 0.25`.
4. Native body-frame branch: nonzero BState enters the occupied formula.
5. Rust render branch: current Rust's BState damage proxy is active for the same
   concrete yellow sample and enters the occupied formula.
6. Screen result: body SHP frame index is consumed by the sprite instance as the
   building body frame.

## Stage Verdicts

### 1. Stock Data

Input: `CAGAS01`.

gamemd data: `CanBeOccupied=yes`, `TechLevel=-1`, `Strength=1000`; thresholds
`ConditionYellow=50%`, `ConditionRed=25%`.

Rust data: the same INI keys are the render inputs through `state.rules`.

Verdict: PASS. The concrete data values match.

### 2. Yellow Damage Tier

Input: `health_current=400`, `health_max=1000`, `ConditionYellow=0.5`,
`ConditionRed=0.25`.

Computed value: `400 / 1000 = 0.4`; yellow tier is true because `0.4 <= 0.5`;
red tier is false because `0.4 > 0.25`.

gamemd: active-YR report verifies the `ConditionRed` and `ConditionYellow`
comparisons in the occupied BState branch.

Rust: `building_bstate_damage_active` returns true; `red_tier` is false and
`yellow_tier` is false because `tech_level > 0` is false for `-1`.

Verdict: PASS for this concrete non-boundary value.

### 3. BState-Gated Occupied Formula Entry

Input: native BState/current building state is explicitly nonzero and occupant
count is positive.

gamemd: `BuildingClass::GetCurrentFrame` enters the `CanBeOccupied` branch only
after the nonzero `BuildingClass+0x534` gate.

Rust: for the concrete yellow sample, the local damage proxy is true and
`rendered_garrison_body_frame_index` calls `building_frame_index`.

Verdict: PASS for this concrete scenario. The broader proxy-vs-native-BState
model is not proven equivalent outside this scoped yellow damaged input.

### 4. Body Frame Formula

Input: `occupant_count=1`, `tech_level=-1`, yellow non-red health.

gamemd computation: positive occupant count sets base frame `2`; red increment
does not fire; buildable-yellow increment does not fire because `TechLevel == -1`;
civilian red collapse does not fire because frame remains `2`.

Rust computation: `base=2`; `red_tier=false`; `yellow_tier=false` because
`tech_level > 0` is false; `base != 3`, so return `2`.

Verdict: PASS. Both output body SHP frame `2`.

## Result

For the scoped scenario, native `gamemd.exe` renders body SHP frame `2`, and
current Rust in `src/app_instances/shp.rs` also renders frame `2`.

Verdict tally: PASS: 4 | FAIL: 0 | UNCHECKED: 0 | NOT-IMPLEMENTED: 0

## Adjacent Findings

- Current Rust models the BState gate as a yellow-health damage proxy rather than
  a native `BuildingClass+0x534` state field. This trace did not expand into
  other BState lifecycle cases.
- Boundary behavior at exactly `ConditionYellow` or `ConditionRed` was not traced
  beyond the existing code/doc evidence; this scenario used a non-boundary yellow
  value.
