# Static Lighting - LightIntensity Zero Allocation Gate Trace

Date: 2026-05-24

Scenario: define/place a building with `LightVisibility=4096` and `LightIntensity` absent or explicitly zero. Verify standard YR creates no building radius light source, then compare against current Rust building-light collection.

Scope is intentionally narrow: one static building light allocation gate. This trace does not evaluate full LightConvert palette output, superweapon lighting modes, spotlight `BuildingLightClass`, or terrain-object light keys.

## Verdict

PASS: 4 | FAIL: 0 | UNCHECKED: 1 | NOT-IMPLEMENTED: 0

Overall status: COMPLETE.

## Pipeline

`INI building type fields -> building enters/completes on map -> gamemd LightSource allocation gate / Rust point-light collection gate -> point-light contribution -> visible local halo`

## Stage Results

### Stage 1 - Data Fields

Input fixture:

```ini
[ZERO]
LightVisibility=4096
; LightIntensity omitted
```

Equivalent explicit-zero fixture:

```ini
[ZERO]
LightVisibility=4096
LightIntensity=0
```

gamemd:

- `BuildingTypeClass_ReadINI_Water @ 0x0045FE50` reads `LightVisibility` into `BuildingTypeClass+0xE30`.
- `LightIntensity` defaults from constructor value `BuildingTypeClass+0xE34 = 0`.
- Explicit `LightIntensity=0` stores `ftol(0.0 * 1000.0 + 0.1) = 0`.
- Active in standard YR: yes, this is the standard `BuildingTypeClass` INI reader.

Rust:

- `src/rules/object_type.rs:1112` parses missing `LightVisibility` default as `5000`, but this fixture explicitly sets `4096`.
- `src/rules/object_type.rs:1113` parses missing `LightIntensity` as `0.0`.
- Existing unit coverage at `src/rules/object_type.rs:1300` verifies missing intensity yields `0.0`.

Computed output:

- gamemd allocation-relevant intensity field: `0`.
- Rust allocation-relevant converted intensity: `light_value_to_units(0.0) = 0`.

Verdict: PASS for this scenario's allocation input.

### Stage 2 - Active YR Allocation Entry Points

gamemd:

- `BuildingClass__Unlimbo @ 0x00440580` checks `*(int *)(Type + 0xE34) != 0` before allocating `operator_new(0x4c)`, calling `LightSourceClass__Constructor`, storing `BuildingClass+0x614`, and enabling the source.
- `BuildingClass__OnConstructionComplete @ 0x00445F80` repeats the same `Type+0xE34 != 0` gate before allocation and enable.
- The checked functions are active standard YR building lifecycle paths, not dormant TS-only code.

Rust:

- Startup/map collection path: `src/map/lighting.rs:389` iterates structures and passes parsed light fields to `point_light_from_object`.
- Live app rebuild path: `src/app_init.rs:184` filters live structures and also calls `point_light_from_object` at `src/app_init.rs:201`.

Computed output:

- gamemd eligible allocation entry points for zero-intensity fixture: `0` source allocations.
- Rust eligible collection outputs for zero-intensity fixture: `0` point lights from both collection paths.

Verdict: PASS for the zero-intensity allocation decision.

### Stage 3 - Gate Function Output

gamemd:

- With `Type+0xE34 = 0`, the checked allocation branch is skipped.
- No `operator_new(0x4c)` is reached.
- No `LightSourceClass__Constructor` is called.
- No non-null `BuildingClass+0x614` light pointer is created for this fixture.

Rust:

- `point_light_from_object` converts the float first: `let intensity = light_value_to_units(intensity)` at `src/map/lighting.rs:426`.
- It immediately returns `None` when `intensity == 0` at `src/map/lighting.rs:427`.
- Existing unit coverage uses a `[ZERO]` fixture with `LightVisibility=4096` and no `LightIntensity`, and `collect_building_lights` returns only the two nonzero-intensity lights: `src/map/lighting.rs:842`.

Computed output:

- gamemd created source count: `0`.
- Rust returned `PointLight` count for the `ZERO` building: `0`.

Verdict: PASS.

### Stage 4 - Point-Light Contribution

gamemd:

- `MAP_LIGHTING_CELL_COMPUTE_00484180_GHIDRA_REPORT.md` verifies point-light contribution only iterates existing active `LightSourceClass` entries.
- Because the zero-intensity building never allocates a source, its additive intensity and RGB contribution to every cell is `0`.

Rust:

- `collect_building_lights`/`collect_live_building_lights` do not produce a `PointLight` for the zero-intensity building.
- `accumulate_point_lights` receives no light for this building, so this building's localized contribution is `0` for every cell.

Computed output:

- gamemd incremental local-light contribution from this building: `0`.
- Rust incremental local-light contribution from this building: `0`.

Verdict: PASS for the scenario-specific light contribution.

### Stage 5 - Absolute Screen Pixels

gamemd:

- Player-visible local halo from the zero-intensity building is absent.

Rust:

- Player-visible local halo from the zero-intensity building is absent by collection.

Unchecked detail:

- This trace did not compute full absolute terrain/object pixel RGB because the scenario intentionally isolates the allocation gate and does not specify a full map `[Lighting]`, LightConvert cache state, detail level, palette, or screenshot capture.

Verdict: UNCHECKED for absolute pixel equality; PASS is limited to the absence of this building's emitted point light.

## Failures

None for the scoped allocation gate.

## Not Implemented

None that affects this scenario. Current Rust does not model gamemd's runtime `BuildingClass+0x614` light-source handle, enable/disable wrappers, or dirty-cell scheduling as an internal shape, but the observable zero-intensity gate output for this scenario is already `0` emitted lights.

## Adjacent Findings

- Nonzero building lights still have broader parity gaps outside this scenario: binary uses lepton-center integer falloff, active/detail gates, post-sum RGB normalization, and LightConvert quantization.
- Runtime lifecycle parity for nonzero building lights remains broader work: gamemd allocates/enables/disables a `LightSourceClass` handle, while Rust rebuilds app-side point lights from live entities.
- `LightVisibility` default alone must never be treated as an allocation gate; only nonzero `LightIntensity` creates the building radius light.

## Top Player-Visible FAIL / NOT-IMPLEMENTED Findings

None for this bounded scenario.

## Sources

- Ghidra read-only spot checks:
  - `BuildingClass__Unlimbo @ 0x00440580`
  - `BuildingClass__OnConstructionComplete @ 0x00445F80`
- Research docs:
  - `docs/research/BUILDINGTYPE_LIGHT_KEYS_READINI_CONSTANTS_GHIDRA_REPORT.md`
  - `docs/research/LIGHTSOURCE_LIFECYCLE_POWER_DAMAGE_SAVELOAD_GHIDRA_REPORT.md`
  - `docs/research/MAP_LIGHTING_CELL_COMPUTE_00484180_GHIDRA_REPORT.md`
- Rust source:
  - `src/rules/object_type.rs:1112`
  - `src/rules/object_type.rs:1113`
  - `src/map/lighting.rs:389`
  - `src/map/lighting.rs:426`
  - `src/map/lighting.rs:427`
  - `src/map/lighting.rs:842`
  - `src/app_init.rs:184`
  - `src/app_init.rs:201`

## Status

COMPLETE.
