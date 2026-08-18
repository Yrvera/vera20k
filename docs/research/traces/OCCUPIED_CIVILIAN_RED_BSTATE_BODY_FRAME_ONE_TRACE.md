# Occupied Civilian Red BState Body Frame One Trace

**Date:** 2026-05-27
**Trace slot:** 3
**Scenario:** In standard Yuri's Revenge, an occupied stock civilian `CanBeOccupied=yes` `TechLevel=-1` building is at red health (`health_ratio <= ConditionRed`) and the native damage/BState path is active. Trace whether `gamemd.exe` computes occupied+red frame `3` and collapses it to body SHP frame `1`, then compare to current Rust in `src/app_instances/shp.rs`.

## Scope

This trace covers only the body SHP frame selected for an occupied red-health civilian garrison while the native BState/damage path is already active. It does not trace garrison entry, BState lifecycle writes, ownership transfer, ActiveAnim slot replacement, muzzle flashes, or damaged-art SHP selection.

## Concrete Inputs

Representative stock building: `CAGAS01`.

- `rulesmd.ini:19302`: `[CAGAS01]`
- `rulesmd.ini:19305`: `TechLevel=-1`
- `rulesmd.ini:19306`: `Strength=1000`
- `rulesmd.ini:19322`: `CanBeOccupied=yes`
- `rulesmd.ini:752`: `ConditionRed=25%`
- `rulesmd.ini:753`: `ConditionYellow=50%`

Concrete red-boundary input used for equality: one occupant, `health_current=250`, `health_max=1000`, `TechLevel=-1`, `ConditionRed=0.25`, `ConditionYellow=0.50`, and BState/damage branch active.

## Pipeline

`BuildingClass::DrawBody` body render -> `BuildingClass::GetCurrentFrame @ 0x0043EF90` -> BState nonzero gate -> `CanBeOccupied` branch -> occupant count sets base frame -> red threshold increment -> civilian frame collapse -> body SHP frame selected -> sprite frame draw.

Current Rust path:

`src/app_instances/shp.rs:138` structure render -> `src/app_instances/shp.rs:141` checks `can_be_occupied` -> `src/app_instances/shp.rs:153` calls `rendered_garrison_body_frame_index` -> `src/app_instances/shp.rs:678` BState/damage proxy gate -> `src/app_instances/shp.rs:707` `building_frame_index` -> `src/app_instances/shp.rs:729` civilian collapse -> frame passed into `ShpSpriteKey`.

## Stage Verdicts

### Stage 1 - Stock Data And Trigger

Input: stock `CAGAS01`, occupied by at least one infantry, health at red boundary `250 / 1000`.

gamemd: `CAGAS01` is active standard YR civilian garrison art/rules data with `TechLevel=-1`, `Strength=1000`, and `CanBeOccupied=yes`. `ConditionRed=25%` makes `250 / 1000 = 0.25` red because the native comparison is inclusive (`<=`).

Rust: `shp.rs` receives `occupant_count=1`, `health_current=250`, `health_max=1000`, `tech_level=-1`, `condition_red=0.25`, `condition_yellow=0.5` from rules/state for the concrete scenario.

Verdict: **PASS** for the traced data values.

### Stage 2 - Native BState-Gated Formula

gamemd evidence:

- `GARRISON_OCCUPIED_BUILDING_VISUAL_STATE_GHIDRA_REPORT.md:44`: `BuildingClass::GetCurrentFrame @ 0x0043EF90` reads `BuildingClass+0x534`; if nonzero and `CanBeOccupied` is true, it computes the garrison body frame.
- `GARRISON_OCCUPIED_BUILDING_VISUAL_STATE_GHIDRA_REPORT.md:54`: the branch compares health ratio against `Rules+0x1708 ConditionRed`.
- `GARRISON_OCCUPIED_BUILDING_VISUAL_STATE_GHIDRA_REPORT.md:56`: the civilian collapse rule is `TechLevel == -1 && frame == 3 -> 1`.
- `GARRISON_OCCUPIED_BUILDING_VISUAL_STATE_GHIDRA_REPORT.md:58`: this `GetCurrentFrame` path is active in standard YR.
- `GARRISON_FRAME_SWAP_GHIDRA_REPORT.md:80`: formula preconditions include `BuildingClass+0x534 != 0` and `Type+0x157B CanBeOccupied != 0`.

Native computation:

1. BState/damage path is active by scenario precondition.
2. `GetOccupantCount() > 0` -> `base = 2`.
3. `health_ratio = 250 / 1000 = 0.25`.
4. `0.25 <= ConditionRed(0.25)` -> red tier true -> `base = 3`.
5. `TechLevel == -1 && base == 3` -> return frame `1`.

Verdict: **PASS**. gamemd output for the concrete frame index is `1`.

### Stage 3 - Current Rust Formula

Rust evidence:

- `src/app_instances/shp.rs:141`: only `CanBeOccupied` structures enter this body-frame path.
- `src/app_instances/shp.rs:153`: calls `rendered_garrison_body_frame_index`.
- `src/app_instances/shp.rs:678`: returns raw frame `0` only when `building_bstate_damage_active` is false.
- `src/app_instances/shp.rs:691`: current BState proxy tests `health_current / health_max <= ConditionYellow`.
- `src/app_instances/shp.rs:707`: `building_frame_index` implements the garrison body formula.
- `src/app_instances/shp.rs:724`: red tier is inclusive (`ratio <= condition_red`).
- `src/app_instances/shp.rs:729`: `tech_level == -1 && base == 3` returns frame `1`.

Rust computation:

1. `building_bstate_damage_active(250, 1000, 0.5)` -> `0.25 <= 0.5` -> true, so the BState-gated formula is entered.
2. `occupant_count=1` -> `base = 2`.
3. `ratio = 250 / 1000 = 0.25`.
4. `red_tier = 0.25 <= 0.25` -> true.
5. `yellow_tier = tech_level > 0 && ratio <= condition_yellow` -> false because `-1 > 0` is false.
6. red tier increments `base` from `2` to `3`.
7. `tech_level == -1 && base == 3` -> return frame `1`.

Verdict: **PASS**. Current Rust output for the concrete frame index is `1`, numerically equal to gamemd.

### Stage 4 - Test Coverage For This Mechanic

Rust evidence:

- `src/app_instances/shp.rs:883`: `occupied_civilian_garrison_bstate_red_collapses_to_frame_one`.
- `src/app_instances/shp.rs:885`: asserts `rendered_garrison_body_frame_index(1, 20, 100, -1, 0.5, 0.25) == 1`.

This test uses a normalized `20 / 100 = 0.20` red sample rather than the stock `CAGAS01` boundary `250 / 1000 = 0.25`, but both are inside `health_ratio <= ConditionRed`.

Verdict: **PASS** for red-tier coverage of the collapse rule; exact stock-strength boundary coverage is not separately present.

### Stage 5 - Player-Visible Pixel Result

Expected player-visible result: the occupied red-health civilian building body uses SHP frame `1`, not frame `3`, so a stock three-frame civilian building does not attempt to show a nonexistent or fallback frame for occupied red damage state.

Rust selected frame equals gamemd selected frame for the concrete scenario. This trace did not launch the renderer, inspect the atlas entry, or compare a screenshot/hash for the final pixels because the swarm slot is read-only except for this report.

Verdict: **UNCHECKED** for final pixel equality beyond the selected frame number.

## Findings

No FAIL or NOT-IMPLEMENTED finding for the requested body-frame collapse. Current Rust matches gamemd's selected body SHP frame for the concrete occupied red-health civilian BState scenario.

Residual risk: current Rust uses a health-threshold proxy for native `BuildingClass+0x534` BState activation (`src/app_instances/shp.rs:691`) rather than a real native BState field. That is adjacent to this trace because the scenario explicitly starts with the native BState path active; it should be traced separately for BState lifecycle parity.

## Verdict Tally

PASS: 4 | FAIL: 0 | UNCHECKED: 1 | NOT-IMPLEMENTED: 0

## References

- `docs/research/GARRISON_OCCUPIED_BUILDING_VISUAL_STATE_GHIDRA_REPORT.md:42`
- `docs/research/GARRISON_OCCUPIED_BUILDING_VISUAL_STATE_GHIDRA_REPORT.md:62`
- `docs/research/GARRISON_FRAME_SWAP_GHIDRA_REPORT.md:80`
- `docs/research/BUILDINGCLASS_DRAWBODY_GHIDRA_REPORT.md:154`
- `ini/rulesmd.ini:752`
- `ini/rulesmd.ini:753`
- `ini/rulesmd.ini:19302`
- `src/app_instances/shp.rs:141`
- `src/app_instances/shp.rs:153`
- `src/app_instances/shp.rs:678`
- `src/app_instances/shp.rs:691`
- `src/app_instances/shp.rs:707`
- `src/app_instances/shp.rs:729`
- `src/app_instances/shp.rs:883`
