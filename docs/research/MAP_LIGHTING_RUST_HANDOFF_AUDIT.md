# MAP_LIGHTING_RUST_HANDOFF_AUDIT

Status: COMPLETE

Scope: current-Rust audit only. This report does not add binary findings; it uses the parent-provided verified facts and local Rust/file inspection.

## Target Question

What should change in the current Rust map-lighting implementation so it aligns with the verified Yuri's Revenge map ambience, lamp `LightSourceClass`, `LightConvertClass`, and `ExtraLight=` facts?

## Non-goals

- Do not investigate new `gamemd.exe` functions.
- Do not edit Rust, INI files, existing docs, or Ghidra state.
- Do not cover Lightning Storm, Tesla/EBOLT weapon visuals, audio ambience, Ion/Nuke/Dominator lighting transitions, or `BuildingLightClass` spotlights except to keep them separated from lamp-post radius lights.
- Do not design a final GPU palette-conversion implementation beyond the handoff needed by the next patch pass.

## Settled Facts Used

- Map ambience is ScenarioClass `[Lighting]`: `Ambient`, `Red`, `Green`, `Blue`, `Ground`, and `Level`.
- Lamp-post-style radius lights are `LightSourceClass` objects stored at `BuildingClass+0x614`; spotlights are separate at `BuildingClass+0x600`.
- `LightSourceClass` allocation in checked `BuildingClass::Unlimbo` is gated by nonzero `LightIntensity`, not `LightVisibility` alone.
- Binary `LightVisibility` default is `5000`; binary `LightIntensity` default is `0`; RGB tint defaults are 1.0-equivalent fixed-point values.
- LightSource falloff is linear in lepton distance.
- `CellClass+0x34` stores cached `LightConvertClass*`; the cache is keyed by normalized RGB triple only, not by cell coordinate or height.
- `ExtraLight=` is not ambience. It is a signed 16-bit building art draw-depth/Z adjustment consumed by `BuildingClass_DrawBody`.

## Current Rust Delta

| Area | Current Rust | Delta / implication |
|---|---|---|
| Map ambience | `src/map/lighting.rs` parses `[Lighting]` into `LightingConfig` and computes `[f32; 3]` per cell (`parse_lighting`, `cell_tint`, `build_lighting_grid`). | High-level ownership is correct, but comments should not claim full original parity until LightConvert normalization/cache and palette application are represented. |
| Lamp collection | `collect_building_lights` skips if `light_visibility <= 0 || light_intensity == 0.0`. | Runtime gate includes intensity, which matches the settled allocation gate. The comment still says "LightVisibility > 0" and should be corrected. |
| LightVisibility default | `ObjectType` docs and parser default `LightVisibility` to `0` in `src/rules/object_type.rs:701` and `src/rules/object_type.rs:1104`. | Binary default is `5000`. Because `LightIntensity` defaults to 0 this usually does not light ordinary stock buildings, but it is wrong for data parity and mods/inheritance cases. |
| Light tints | `LightIntensity` and RGB tints are parsed through `IniSection::get_f32` in `src/rules/object_type.rs:1105`. | `get_f32` uses Rust `parse::<f32>()` at `src/rules/ini_parser.rs:77`, so comma-decimal stock/mod values such as `0,01` fail to parse and silently fall back. |
| Light accumulation | `accumulate_point_lights` adds linear `f32` RGB contributions directly into `LightingGrid`. | Good as a provisional scalar model, but final screenshot parity needs the LightConvert cache/profile shape keyed by normalized RGB triple. |
| LightConvert cache | There is no `LightConvertClass`-style cache in the current Rust surface. Render consumers directly read `LightingGrid` in `src/app_instances/shp.rs`, `overlays.rs`, `units.rs`, and `bridges.rs`. | Add a render-facing light profile/cache or equivalent indexed normalized RGB profile before treating per-cell lighting as final parity. |
| Load/update ownership | `src/app_init.rs:339` builds lighting once, then accumulates lamps and applies `ExtraLight`. | Static map load path is a reasonable first patch target. Dynamic dirty-cell scheduling from light changes/destruction remains separate work. |
| ExtraLight | `src/rules/art_data.rs:96` documents it as ambient light, `src/map/lighting.rs:223` applies it as RGB brightness, and `app_init.rs:360` calls it. | This is parity-wrong. Keep parsing the INI key, reinterpret/rename internally as a signed building Z/depth adjustment, remove it from map RGB lighting. |

## Recommended Patch Order

1. Remove `ExtraLight=` from map RGB lighting.
   - Delete or retire `src/map/lighting.rs::apply_extra_light`.
   - Remove the `app_init.rs` call at `src/app_init.rs:361`.
   - Replace `test_extra_light_boost` with a negative lighting test.
   - Keep parsing `ExtraLight=`, but move its semantic comment away from ambience and toward signed depth/Z adjustment.

2. Correct object light defaults and parser behavior.
   - Change `LightVisibility` default from `0` to `5000`.
   - Keep `LightIntensity == 0.0` as the light-allocation/collection gate.
   - Add coverage for inherited/default visibility plus nonzero intensity.
   - Add INI float parsing coverage for comma decimal values if stock or mod light tint values use that format in loaded data.

3. Split the provisional `LightingGrid` model from final render profiles.
   - Preserve a clear map/cell light computation stage.
   - Add a deterministic `LightProfile`/`LightConvertCache`-equivalent keyed by normalized RGB triple.
   - Ensure default unlit RGB resolves to one shared profile.
   - Avoid keying by `(rx, ry)`, height, or raw `f32` values.

4. Update immediate render consumers after the cache/profile exists.
   - Current consumers read raw `[f32; 3]` from `state.lighting_grid`.
   - SHP, overlay, bridge, and VXL/unit paths should receive either the cached profile id plus scalar light fields or an equivalent renderer-facing abstraction.
   - Do not mix this with building spotlight work; `BuildingLightClass` is a separate path.

5. Apply `ExtraLight=` to building draw depth.
   - Use the parsed signed value on the building SHP/body depth path, not map lighting.
   - The current obvious depth target is around `src/app_instances/shp.rs:223`, where building sprite depth is computed before bridge/depth bias.
   - Keep exact placement subject to the building body/depth implementation pass.

6. Leave dynamic lighting invalidation for a follow-up patch.
   - Static map-load lights can be fixed first.
   - Power toggles, building destruction, and dirty-cell recomputation should wait for the dirty scheduling report/handoff.

## Acceptance Tests

Concrete tests to add or replace:

- `map_lighting_does_not_apply_extra_light_to_rgb_grid`
  - Given a building art entry with `ExtraLight=350`, building-cell RGB remains unchanged after map lighting build.

- `art_extra_light_parses_as_signed_depth_adjustment`
  - `ExtraLight=-100` and `ExtraLight=350` are preserved as signed values and are not scaled by `/1000`.

- `object_type_light_visibility_defaults_to_5000`
  - A rules object without `LightVisibility=` has `light_visibility == 5000`.

- `building_light_collection_gates_on_nonzero_intensity`
  - Default visibility plus `LightIntensity=0` produces no `PointLight`; default visibility plus nonzero intensity produces one.

- `ini_get_f32_accepts_comma_decimal_light_values`
  - A light tint/intensity value such as `0,01` parses as `0.01` where the engine expects a scalar float.

- `lightconvert_cache_reuses_default_profile_for_unlit_cells`
  - Multiple unlit/default cells resolve to the same render light profile.

- `lightconvert_cache_key_ignores_cell_coordinate`
  - Two cells with the same normalized RGB triple but different coordinates share a cache entry.

- `lightconvert_cache_key_is_normalized_rgb_not_raw_f32`
  - Numerically equivalent or same-quantized RGB inputs do not create duplicate profiles.

- `building_extra_light_depth_bias_affects_sprite_sort_only`
  - A building with `ExtraLight=350` changes computed building body depth/Z adjustment while its cell RGB profile stays unchanged.

## Negative Facts / Do Not Do

- Do not implement lamp posts through Lightning Storm, weather, audio ambience, or weapon visuals.
- Do not allocate or collect light sources just because `LightVisibility` is nonzero; the checked live allocation gate is nonzero `LightIntensity`.
- Do not treat `ExtraLight=` as RGB brightness, radius light, `LightSourceClass`, `LightConvertClass`, or palette ambience.
- Do not divide `ExtraLight=` by `1000`; the verified consumer uses the raw signed value as depth/Z adjustment.
- Do not merge `BuildingClass+0x600` spotlight behavior into map lamp radius lights at `+0x614`.
- Do not key the render light cache by cell coordinate, height, or a full raw `f32` bundle.
- Do not leave Rust comments claiming exact original light calculation where the LightConvert/cache/profile layer is still absent.

## Remaining Uncertainty

- Exact fixed-point clamp/normalization boundaries should come from the dedicated LightSource/light-compute reports before final screenshot parity claims.
- Dynamic dirty-cell scheduling for light enable/disable/destruction is outside this audit.
- Exact insertion point for `ExtraLight=` depth adjustment depends on the building body/depth pass and should be verified against the draw-order reports before patching.
- Ion/Nuke/Dominator ambient transitions are parsed by the binary but intentionally out of scope here.

## Handoff Summary

First patch should remove the actively wrong `ExtraLight` RGB behavior and correct the parser/default comments. Second patch should introduce the LightConvert-style render profile cache so the renderer stops treating lighting as only a per-cell `[f32; 3]`. Dynamic updates and spotlights should remain separate follow-up work.
