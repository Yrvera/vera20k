---
name: Healthy occupied CAGAS01 body frame zero trace
scenario: Healthy occupied CAGAS01 body SHP frame selection
date: 2026-05-27
status: PASS
---

# Healthy Occupied CAGAS01 Body Frame Zero Trace

## Scope

Concrete scenario only: standard Yuri's Revenge `CAGAS01`, healthy/idle,
`CanBeOccupied=yes`, at least one occupant, no live active building anim slot.
Trace the body SHP frame selected for rendering and compare it to current Rust
after the visual-state fix in `src/app_instances/shp.rs`.

Non-goals: damaged occupied frames, active anim replacement semantics, muzzle
flash drawing, occupant firing, garrison entry/exit, ownership reconciliation.

## Sources

- `ini/rulesmd.ini:19302..19325`: `CAGAS01`, `TechLevel=-1`,
  `Strength=1000`, `CanBeOccupied=yes`, `MaxNumberOccupants=10`,
  `CanOccupyFire=yes`.
- `ini/artmd.ini:8019..8028`: `CAGAS01` art entry has `;ActiveAnim=CAWSH12A`
  commented out and no active building anim key in the scoped lines.
- `docs/research/GARRISON_OCCUPIED_BUILDING_VISUAL_STATE_GHIDRA_REPORT.md:44..60`:
  verified `BuildingClass::GetCurrentFrame @ 0x0043EF90` behavior and active
  YR status.
- `docs/research/GARRISON_FRAME_SWAP_GHIDRA_REPORT.md:9..11`,
  `143..156`, `189..221`: verified active function, BState gate, and static
  stock civilian no-active-anim context.
- `src/app_instances/shp.rs:153..160`, `670..700`, `867..872`: current Rust
  body frame call, BState/damage proxy, and focused regression test.

## Pipeline

`rulesmd/artmd data -> occupant cargo count -> body-frame render query -> body SHP frame index -> screen body image`

## Entry Points

1. Building body sprite emission: `src/app_instances/shp.rs:153..160`.
   Fires during render sprite construction when the object art is a building
   SHP and the object is garrisonable.
2. Building anim emission: `src/app_instances/shp.rs:286..301`.
   Adjacent only for this trace; `CAGAS01` has no live active anim slot, so it
   should not affect the scoped body SHP frame.

Coverage: the scoped body-frame path is covered. Runtime map spawn and garrison
entry paths are outside this trace because the initial condition explicitly
starts with at least one occupant.

## Concrete Values

Scenario inputs:

- `object = CAGAS01`
- `TechLevel = -1`
- `Strength/health_max = 1000` in INI; Rust focused unit test uses normalized
  equivalent `100/100`
- `health ratio = 1.0`
- `occupant_count >= 1`; focused Rust computation uses `1`
- `ConditionYellow = 0.5`
- `ConditionRed = 0.25`
- Native healthy/idle BState field `BuildingClass+0x534 = 0`

## Stage Verdicts

### Stage 1 - Stock Data

`CAGAS01` is a stock static civilian garrison candidate: `TechLevel=-1`,
`CanBeOccupied=yes`, `MaxNumberOccupants=10`, and `CanOccupyFire=yes`.
Its art entry has the active anim key commented out.

Verdict: PASS for scenario classification.

### Stage 2 - gamemd Body Frame

Verified active YR function: `BuildingClass::GetCurrentFrame @ 0x0043EF90`.
For `BuildingClass+0x534 == 0`, the function returns current body frame
`+0xF8` before the `CanBeOccupied` branch. For a healthy idle static building,
`+0xF8` is normally `0`.

Concrete gamemd output for this scenario:

```text
BState/current anim state = 0
current body phase +0xF8 = 0
occupant_count >= 1 is not read by this branch
body SHP frame = 0
```

Verdict: PASS. Active standard YR evidence is cited in the verified Ghidra
reports.

### Stage 3 - Rust Body Frame

Current Rust calls `rendered_garrison_body_frame_index(...)` from
`src/app_instances/shp.rs:153..160`. The helper returns `0` whenever
`building_bstate_damage_active(...)` is false. For the focused healthy input:

```text
occupant_count = 1
health_current / health_max = 100 / 100 = 1.0
condition_yellow = 0.5
building_bstate_damage_active = 1.0 <= 0.5 = false
body SHP frame = 0
```

Focused verification run:

```text
cargo test -q healthy_occupied_static_civilian_garrison_render_frame_stays_zero
running 1 test
test result: ok. 1 passed
```

Verdict: PASS. Rust output `0` equals gamemd output `0`.

### Stage 4 - Screen Result

The body sprite uses frame `0`, not frame `2`. Because `CAGAS01` has no active
building anim slot in stock `artmd.ini`, there is no separate healthy occupied
anim replacement to alter the scoped static body result.

Verdict: PASS for body SHP frame selection. Full pixel screenshot validation was
not run in this trace.

### Stage 5 - Native BState Model Fit

Native uses an explicit `BuildingClass+0x534` state field. Current Rust uses a
health-threshold proxy for the immediate body-frame gate. For this healthy/idle
scenario the values are numerically equal (`false` gate in Rust, `0` BState in
gamemd), but the broader field lifecycle is not proven by this trace.

Verdict: UNCHECKED outside the concrete healthy/idle scenario.

## Failures

None for the scoped scenario.

## Not Implemented

No scoped body-frame behavior is missing for the healthy occupied `CAGAS01`
case. A native building BState/live-slot state model remains future work outside
this trace.

## Timing

gamemd queries `GetCurrentFrame` during building body rendering. Rust computes
the body frame during sprite emission for the render pass. No extra tick delay
or deferred state write is involved in the scoped frame selection. Exact global
render ordering beyond this body-frame value was not traced here.

## Adjacent Findings

- Damaged occupied body frames are intentionally not traced here.
- `ActiveAnimGarrisoned` slot replacement is intentionally not traced here.
- `CAGAS01` has no live stock active building anim slot, so the adjacent anim
  replacement path should be inert for this concrete stock scenario.

## Verdict Tally

PASS: 4 | FAIL: 0 | UNCHECKED: 1 | NOT-IMPLEMENTED: 0

