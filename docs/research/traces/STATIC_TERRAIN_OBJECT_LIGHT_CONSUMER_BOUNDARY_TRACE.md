# Static Terrain Object Light Consumer Boundary Trace

Date: 2026-05-24

Scenario: one real building lamp next to one terrain object/tree. The concrete stock data used here is `INGALITE` as the building `LightSource` emitter and `TIBTRE01` as the adjacent terrain object. The trace checks that `INGALITE` lights the tree's cell, while `TIBTRE01` does not create or expand any light area from its own `Light*` INI lines.

Report status: COMPLETE for source ownership and render-consumer boundary; final pixel equality remains UNCHECKED because this trace did not run a gamemd screenshot/blitter comparison.

## Pipeline

`rulesmd.ini` data -> building/terrain type parsing -> map/sim instance ownership -> cell-light computation -> terrain object draw input -> screen pixels.

## Concrete Data

- Building emitter: `INGALITE`, `ini/rulesmd.ini:17284`, with `LightVisibility=5000`, `LightIntensity=0.2`, `LightRedTint=0.05`, `LightGreenTint=0.05`, `LightBlueTint=0.01`.
- Terrain object: `TIBTRE01`, `ini/rulesmd.ini:28109`, with stray terrain-section `LightVisibility=4000`, `LightIntensity=0.01`, `LightRedTint=0.01`, `LightGreenTint=1.5`, `LightBlueTint=0.01`.
- Placement model for numeric checks: `INGALITE` at cell `(10,10)`, `TIBTRE01` at adjacent cell `(11,10)`, both ground level. Cell centers are separated by `256` leptons.
- Default lighting model used for the computed subset: `Ambient=1.0`, `Red=1.0`, `Green=1.0`, `Blue=1.0`, `Ground=0.20`, `Level=0.032`, so base ground scalar is `800` milli-units.

## Stage Table

| Stage | Boundary | gamemd evidence and output | Rust evidence and output | Verdict |
|---|---|---|---|---|
| 1. Terrain light-key parsing | Does `TIBTRE01` create light fields from its `Light*` lines? | `TerrainTypeClass__ReadINI_Full @ 0x0071DEA0` reads terrain fields but not `LightVisibility`, `LightIntensity`, or `Light*Tint`; output terrain-owned light fields: `0`. Active standard YR per `TERRAIN_OBJECT_LIGHT_KEYS_AND_LIGHTSOURCE_OWNERSHIP_GHIDRA_REPORT.md`. | `TerrainObjectType` parses only name, `SpawnsTiberium`, `IsAnimated`, animation timing/probability at `src/rules/terrain_object_type.rs:16`; terrain-owned light fields: `0`. | PASS |
| 2. LightSource ownership | How many sources does the tree add? | `LightSourceClass__Constructor @ 0x00554760` callers are building Unlimbo, building construction complete, and RadSite activation only; no terrain caller. Output from `TIBTRE01`: `0` sources. | `rebuild_lighting_grid_from_sim` calls `collect_live_building_lights`; that filters `EntityCategory::Structure` only at `src/app_init.rs:191`. `TIBTRE01` terrain object is not a structure source. Output from `TIBTRE01`: `0` sources. | PASS |
| 3. Lamp source ownership | Does the real lamp add one source? | `INGALITE` is a building type with nonzero `LightIntensity`; building LightSource paths are active in YR. Output: one active building/radius source when online and detail gate passes. | `point_light_from_object` returns `Some(PointLight)` for nonzero intensity and radius at `src/map/lighting.rs:419`; `INGALITE` fields produce one point light. | PASS |
| 4. Adjacent-cell lamp contribution subset | Does the lamp change the tree cell without tree expansion? | For adjacent cell: distance `256`, radius `5000`, falloff factor `(5000-256)*1000/5000 = 948`. `INGALITE` units are intensity `200`, tints `[50,50,10]`; RGB additions are `[9,9,1]` milli-units. `TIBTRE01` adds `[0,0,0]`. | `accumulate_point_lights` uses lepton centers, integer sqrt, inclusive radius, and summed units at `src/map/lighting.rs:463`; same concrete subset gives base `[800,800,800]` plus `[9,9,1]` -> compat tint `[0.809,0.809,0.801]`. No terrain-object source path exists, so adding the tree does not change the lit area. | PASS |
| 5. Terrain object draw input | Does the tree consume the resolved cell lighting exactly like gamemd? | `TerrainClass__Draw_It @ 0x0071C250` lazy-inits cell lighting and passes scalar `Cell+0x10C` for normal terrain objects, or `Cell+0x10A` for the `SpawnsTiberium` branch. `TIBTRE01` has `SpawnsTiberium=yes`, so the branch is relevant. | `build_overlay_instances` passes `state.lighting_grid.terrain_object_tint_at((obj.rx,obj.ry))` to `SpriteInstance.tint` at `src/app_instances/overlays.rs:433`; `terrain_object_tint_at` returns the collapsed RGB/common tint at `src/map/lighting.rs:245`. There is no `+0x10A`/`+0x10C` scalar bundle or TIBTRE branch. | NOT-IMPLEMENTED |
| 6. Final screen pixels | Does the tree sprite pixel output match gamemd? | Gamemd final output depends on `CC_Draw_Shape`, selected scalar, and LightConvert/blitter tables. This trace did not run a retail screenshot capture. | Rust final output depends on shader multiplication by `SpriteInstance.tint`; no screenshot capture was run here. | UNCHECKED |

## Findings

### PASS: terrain object is not an emitter

For this exact `INGALITE` plus `TIBTRE01` scenario, the terrain object itself contributes zero light sources in both engines. The old tempting interpretation, "TIBTRE Light* lines make ore trees glow", is wrong for standard YR. Rust currently matches the ownership boundary because terrain objects are not collected by `collect_live_building_lights`.

### PASS: real lamp affects the tree cell

The real building lamp affects the adjacent tree cell through the existing point-light path. For the concrete adjacent-cell subset, both the verified binary formula and current Rust produce a low additive contribution from `INGALITE`, while the tree contributes zero.

### NOT-IMPLEMENTED: exact terrain-object draw-light consumer

The player-visible risk is not source ownership. It is exact sprite lighting. Gamemd terrain-object drawing consumes branch-selectable cell scalar fields, with `TIBTRE01` taking the `SpawnsTiberium`/`Cell+0x10A` branch. Rust currently collapses terrain object lighting to a single RGB tint through `terrain_object_tint_at`, so the tree will be lit by existing cell lighting but not with the same draw input model as gamemd.

## Verdict Tally

PASS: 4

FAIL: 0

UNCHECKED: 1

NOT-IMPLEMENTED: 1

## Player-Visible Fail / Not-Implemented Ranking

1. Stage 5 - terrain object draw input: `TIBTRE01` should consume gamemd's branch-specific scalar cell light (`Cell+0x10A` for `SpawnsTiberium=yes`), but Rust uses one collapsed RGB tint at `src/app_instances/overlays.rs:433` and `src/map/lighting.rs:245`; player-visible risk is tree/terrain-object brightness or color not matching nearby lamp lighting. Evidence: `TerrainClass__Draw_It @ 0x0071C250`, `LIGHTING_DRAW_CONSUMERS_CELL_FIELDS_GHIDRA_REPORT.md`.

## Adjacent Findings

- Current Rust source ownership is correct for this scenario, but broader static-lighting parity still depends on implementing a render-facing cell light bundle rather than only `[f32; 3]` tint.
- This trace did not check terrain tile, overlay, techno, animation, or LightConvert palette-table equality.
- This trace did not check `DetailLevel` option gating beyond using the default/high-detail active-source case.

## Sources

- `docs/research/TERRAIN_OBJECT_LIGHT_KEYS_AND_LIGHTSOURCE_OWNERSHIP_GHIDRA_REPORT.md`
- `docs/research/LIGHTING_DRAW_CONSUMERS_CELL_FIELDS_GHIDRA_REPORT.md`
- `docs/research/MAP_LIGHTING_CELL_COMPUTE_00484180_GHIDRA_REPORT.md`
- `src/app_instances/overlays.rs`
- `src/map/lighting.rs`
- `src/app_init.rs`
- `src/rules/terrain_object_type.rs`
- `ini/rulesmd.ini`
