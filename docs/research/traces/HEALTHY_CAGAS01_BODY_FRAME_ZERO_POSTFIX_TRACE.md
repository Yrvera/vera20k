---
name: Healthy CAGAS01 body frame zero postfix trace
scenario: Healthy occupied stock CAGAS01 body frame after visual-state postfix
status: COMPLETE
---

# Healthy CAGAS01 Body Frame Zero Postfix Trace

Scope: standard Yuri's Revenge stock `CAGAS01`, healthy/idle, `CanBeOccupied=yes`,
at least one occupant, no live active building anim slot. Verify current Rust
after the visual-state postfix renders body SHP frame `0`, not occupied frame
`2`.

## Evidence Inputs

- Stock YR rules data: `ini/rulesmd.ini:19302..19325` defines `CAGAS01` with
  `TechLevel=-1`, `Strength=1000`, `CanBeOccupied=yes`,
  `MaxNumberOccupants=10`, and `CanOccupyFire=yes`.
- Stock YR art data: `ini/artmd.ini:8019..8041` defines `CAGAS01`; its only
  `ActiveAnim` entry is commented (`;ActiveAnim=CAWSH12A`), so this concrete
  stock object has no live active building anim slot.
- Native active-YR evidence:
  `docs/research/GARRISON_OCCUPIED_BUILDING_VISUAL_STATE_GHIDRA_REPORT.md:44..60`
  verifies `BuildingClass::GetCurrentFrame @ 0x0043EF90` reads
  `BuildingClass+0x534` before the `CanBeOccupied` branch. If `+0x534 == 0`,
  it returns the current body frame `+0xF8` and does not inspect occupancy.
  The same report marks this function active in standard YR body rendering.
- Current Rust source inspected after the fix:
  `src/app_instances/shp.rs:141..160`, `src/app_instances/shp.rs:745..775`,
  and `src/app_instances/shp.rs:790..807`.

## Pipeline

1. Data load: `CAGAS01` is a stock garrisonable civilian building.
2. Render body selection: Rust enters the `CanBeOccupied` body-frame path.
3. Visual-state gate: healthy input decides whether the occupied/damaged frame
   formula is active.
4. Body frame output: selected body frame is passed to the SHP sprite key.
5. Active anim overlay: no live active slot exists for this stock object.

## Stage Verdicts

### 1. Stock Data

Input: `CAGAS01`.

Native output: `CanBeOccupied=1`, `TechLevel=-1`, `Strength=1000`, active
building anim slots for this scenario `0`.

Rust-visible input from stock INI: same concrete values; commented
`;ActiveAnim=CAWSH12A` is not an active key.

Verdict: PASS. Literal scoped values match: `CanBeOccupied 1 == 1`,
`TechLevel -1 == -1`, `Strength 1000 == 1000`, active slots `0 == 0`.

### 2. Healthy Visual-State Gate

Concrete input: one occupant, health `1000 / 1000`, `ConditionYellow=0.5`,
`ConditionRed=0.25`, healthy/idle native BState `BuildingClass+0x534 = 0`.

Native output: `GetCurrentFrame` sees `+0x534 == 0`, skips the
`CanBeOccupied` occupied-frame branch, and returns current body frame `+0xF8`.
For the stock healthy idle body this is frame `0`.

Rust output:

```text
building_bstate_damage_active(1000, 1000, 0.5)
= (1000 / 1000) <= 0.5
= 1.0 <= 0.5
= false
rendered_garrison_body_frame_index(...) returns 0
```

Verdict: PASS for this concrete post-fix scenario. Literal output is
`0 == 0`; Rust does not produce occupied body frame `2`.

### 3. Occupant Count Does Not Affect Healthy Body Frame

Concrete input: `occupant_count=1`.

Native output: because `+0x534 == 0`, the occupant-count call in the
`CanBeOccupied` branch is not reached. Effective rendered frame remains `0`.

Rust output: because `building_bstate_damage_active(...)` returned false,
`building_frame_index(...)` is not called. Effective rendered frame remains `0`.

Verdict: PASS. Literal rendered frame remains `0 == 0` with one occupant.

### 4. Active Building Anim Overlay

Concrete input: stock `CAGAS01` has no live active building anim slot because
`ActiveAnim` is commented in `artmd.ini`.

Native output: no active/garrisoned active anim overlay is emitted for this
stock idle body scenario.

Rust output: no parsed active building anim slot exists for this object in this
scenario, so no active/garrisoned overlay contributes to the body-frame result.

Verdict: PASS. Literal overlay count for this scoped active slot is `0 == 0`.

### 5. Runtime Pixel Capture

No live renderer screenshot or frame-capture comparison was run in this
read-only trace slot.

Verdict: UNCHECKED. Source-level output is computed as frame `0`, but a
runtime pixel capture was not produced in this trace.

## Failures

None in the scoped post-fix scenario.

## Not Implemented

None in the scoped post-fix scenario.

## Adjacent Findings

- Rust still uses a health-threshold proxy for the native `BuildingClass+0x534`
  visual-state field. That does not change this healthy/idle `CAGAS01` output,
  but full parity should eventually model or pass the native visual state rather
  than deriving it only from health.
- Damaged/yellow/red body-frame behavior and active anim replacement metadata
  are outside this single slot.

## Verdict Tally

PASS: 4 | FAIL: 0 | UNCHECKED: 1 | NOT-IMPLEMENTED: 0

Status: COMPLETE
