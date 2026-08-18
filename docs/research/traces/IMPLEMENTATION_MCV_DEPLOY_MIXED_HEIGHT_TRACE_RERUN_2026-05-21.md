# Implementation Trace Rerun: MCV Deploy over Mixed Terrain Heights

Date: 2026-05-21

Scope: rerun the focused implementation trace after the intended MCV mixed-height deploy fix.

Concrete fixture:

- Unit/building data: `AMCV -> GACNST`.
- Rust MCV cell: `(20,22)`.
- Rust deploy origin: `(19,21)` from `deploy_origin_from_center`.
- `GACNST` foundation in the current test fixture: `4x3`.
- Mixed-height footprint input: origin height defaults to `0`; `(20,21)=1`; all cells otherwise clear.

## Pipeline

Deploy command -> deploy target lookup -> foundation origin -> footprint validation -> MCV removal/building spawn -> visible result.

## Evidence Used

- Rust source:
  - `src/sim/world/world_spawn.rs:495-590`
  - `src/sim/deploy_tests.rs:230-253`
- Test execution:
  - `target/debug/deps/vera20k-05fff7103a8c59c0.exe deploy_mcv --nocapture`
- Ghidra read-only:
  - `UnitClass__Deploy @ 0x007393C0`
  - `BuildingTypeClass__CanBePlacedAt @ 0x0045EE70`
  - `BuildingClass__Unlimbo @ 0x00440580`

## Stage Trace

### Stage 1 - Deploy Target and Origin

Rust resolves the source unit's `DeploysInto=` target and computes the `GACNST` origin from MCV `(20,22)` to `(19,21)`.

gamemd `UnitClass__Deploy @ 0x007393C0` reads the deploy target and applies the large-foundation NW offset before calling the new building's placement virtual.

Verdict: `PASS` for the traced origin behavior.

### Stage 2 - Mixed-Height Footprint Gate

Rust current source still computes a reference height:

- `src/sim/world/world_spawn.rs:527`: `ref_height = height_map[(rx,ry)].unwrap_or(z)`
- `src/sim/world/world_spawn.rs:532-539`: any footprint cell whose height differs from `ref_height` returns `false`

For the concrete fixture:

- origin `(19,21)` height = `0`
- cell `(20,21)` height = `1`
- Rust output: `deploy_mcv` returns `false`

gamemd `BuildingTypeClass__CanBePlacedAt @ 0x0045EE70` walks foundation offsets and checks bounds, overlay/building contents, first object, upgrade/allied movable overlap, and scatter side effects. The read-only decompile has no terrain-height read and no same-height equality gate. `BuildingClass__Unlimbo @ 0x00440580` also has no all-foundation-cells-same-height rejection before the normal placement branch.

Verdict: `FAIL`.

### Stage 3 - MCV Despawn / ConYard Spawn

Rust returns before `despawn_entity` and before `spawn_object_at_height`.

For the concrete fixture:

- Rust output: AMCV remains; no `GACNST` spawns.
- gamemd expected output: no same-height reject; otherwise-clear placement proceeds to building creation/Unlimbo.

Verdict: `FAIL`.

### Stage 4 - Regression Test State

Current Rust test coverage encodes the wrong behavior:

- `src/sim/deploy_tests.rs:231`: `deploy_mcv_rejects_mixed_height_clear_foundation`
- The assertion at `src/sim/deploy_tests.rs:248-251` requires `applied == false`.

Direct test binary execution:

```text
running 4 tests
test sim::deploy_tests::deploy_mcv_rejects_mixed_height_clear_foundation ... ok
test sim::deploy_tests::deploy_mcv_uses_gamemd_large_foundation_origin_offset ... ok
test sim::deploy_tests::deploy_mcv_rejects_structure_in_rightmost_foundation_column ... ok
test sim::world::tests::test_deploy_mcv_replaces_vehicle_with_conyard ... ok
```

Verdict: `FAIL` for parity coverage. The test suite currently protects the non-parity behavior.

## Failures

1. Rust still rejects mixed-height MCV deploy footprints.
2. Rust therefore leaves the AMCV undeployed and does not spawn `GACNST`.
3. Current test coverage asserts rejection instead of acceptance, so the intended fix is not present in the current worktree.

## Not Implemented

No new not-implemented stage was found in this rerun. The earlier `EVA_CannotDeployHere` gap remains adjacent only when a deploy legitimately fails; for this scenario the failure itself should not happen.

## Verdict Tally

PASS: 1 | FAIL: 3 | UNCHECKED: 0 | NOT-IMPLEMENTED: 0

## Status

COMPLETE - current worktree still fails this trace.
