# Map Lighting Implementation Spec

Date: 2026-05-22
Status: COMPLETE
Scope: Rust implementation handoff for ordinary map lighting, lamp/radius lights, render-facing cell-light profiles, `ExtraLight=`, and known parser/default fixes.

## Target Question

What should the Rust implementation change, in what order, to match verified Yuri's Revenge ordinary map lighting and lamp-post ambience behavior without mixing in Lightning Storm, weapon visuals, audio ambience, or spotlight beams?

## Non-goals

- Do not perform new binary research.
- Do not patch Rust, INI files, existing research reports, Ghidra state, or `.swarm-claims.md`.
- Do not cover Lightning Storm/Ion/Nuke/Dominator transition timelines beyond keeping their fields separate from ordinary map ambience.
- Do not implement `BuildingLightClass` spotlight/searchlight beams in the lamp ambience patch.
- Do not design byte-exact `ConvertClass`/blitter table internals; this spec only requires the renderer-facing cache/profile shape already verified.
- Do not require dynamic dirty-cell lighting in the first static-load parity patch.

## Verified Inputs Used

- `SCENARIO_LIGHTING_FIELDS_00689E90_GHIDRA_REPORT.md`
- `SCENARIO_LIGHTING_DEFAULT_RESET_PATH_GHIDRA_REPORT.md`
- `MAP_LIGHTING_CELL_COMPUTE_00484180_GHIDRA_REPORT.md`
- `MAP_LIGHTCONVERT_CACHE_00483E30_00544E70_GHIDRA_REPORT.md`
- `LIGHTING_DRAW_CONSUMERS_CELL_FIELDS_GHIDRA_REPORT.md`
- `LIGHTSOURCE_DIRTY_SCHEDULING_00554AF0_00554D50_GHIDRA_REPORT.md`
- `LIGHTSOURCE_QUEUED_MODE_CALLER_CENSUS_GHIDRA_REPORT.md`
- `BUILDINGTYPE_EXTRALIGHT_0X1548_CONSUMER_GHIDRA_REPORT.md`
- `EXTRALIGHT_DRAWBODY_Z_DELTA_DETAILS_GHIDRA_REPORT.md`
- `BUILDINGLIGHTCLASS_SPOTLIGHT_PATH_GHIDRA_REPORT.md`
- `LIGHT_RULES_ART_PARSER_DEFAULTS_GHIDRA_REPORT.md`
- `MAP_LIGHTING_RUST_HANDOFF_AUDIT.md`
- Current Rust surfaces: `src/map/lighting.rs`, `src/app_init.rs`, `src/rules/object_type.rs`, `src/rules/art_data.rs`, `src/rules/ini_parser.rs`, `src/app_instances/*.rs`, `src/render/batch.rs`, `src/map/terrain.rs`.

## Settled Model

Ordinary map lighting is a ScenarioClass-owned cell-lighting system. It reads ordinary `[Lighting]` keys `Ambient`, `Red`, `Green`, `Blue`, `Ground`, and `Level`, using reset defaults when keys are missing. The ordinary brightness formula is additive:

```text
top/common-ish brightness = ambient + local_light_intensity + level * cell_level - ground
bottom-ish brightness     = ambient + local_light_intensity + level * (cell_level + 4) - ground
```

Map author units are converted into integer fields first: `Ambient/Red/Green/Blue` scale by `100`; `Ground/Level` scale by `250`; cell compute then works in `1000 == 1.0` units. Missing ordinary keys preserve binary reset defaults: `Ambient/R/G/B=1.00`, `Ground=0.20`, `Level=0.032`.

Lamp posts and similar ambience lights are building/radiation `LightSourceClass` radius lights, not weather or superweapon logic. Building light allocation/collection is gated by nonzero `LightIntensity`; `LightVisibility` defaults to `5000` but does not create light by itself. Point-light contribution uses lepton-center integer math, inclusive radius tests, signed negative-light support, detail-level gating, post-sum RGB normalization, and LightConvert profile selection.

Render consumers do not receive only a per-cell `[f32; 3]` tint. Binary cells store a `LightConvertClass*` equivalent plus scalar fields:

- `Cell+0x34`: cached LightConvert profile pointer.
- `Cell+0x10A`: alternate/top scalar used by several overlay, terrain-object, anim, and building depth/Z-style paths.
- `Cell+0x10C`: common scalar consumed by TMP terrain, overlays, terrain objects, Techno SHPs, anims, and queued draw paths.
- `Cell+0x10E`: alternate/bottom scalar used by overlay/special branches.
- `Cell+0x110/+0x112/+0x114`: normalized RGB cache-key mirror.

`ExtraLight=` is not ambience. It is a signed `BuildingTypeClass+0x1548` draw-depth/Z delta added to signed `Cell+0x10A` in `BuildingClass_DrawBody`. It must not brighten/darken RGB cells and must not be divided by `1000`.

## Current Rust Mismatch

- `src/map/lighting.rs` computes direct per-cell RGB tint using `ambient * (1.0 - ground) + level * z`; binary uses additive `ambient + level*z - ground`.
- `LightingConfig::default` uses `Ground=0.0`; binary missing-key default is external `0.20`.
- `LightingGrid` is `HashMap<(u16,u16), [f32;3]>`; binary has a cell-light bundle with profile key plus scalar fields.
- `accumulate_point_lights` uses cell-space float distance, per-contribution channel clamps, and no detail gate; binary uses lepton-center integer falloff, sums first, then normalizes/clamps.
- `apply_extra_light` applies `ExtraLight / 1000.0` to RGB lighting; binary uses raw signed `ExtraLight` for building draw-depth/Z only.
- `src/rules/object_type.rs` defaults `LightVisibility` to `0`; binary default is `5000`.
- `src/rules/ini_parser.rs::get_f32` uses Rust full-string parsing. For stock malformed light values such as `0,01`, gamemd `sscanf("%f")` parses the numeric prefix as `0.0`; Rust currently falls back to the supplied default.
- Render instance builders consume only tint arrays, while binary render paths use branch-selectable scalar fields and a LightConvert profile.
- Lighting is baked at startup in `src/app_init.rs`; verified standard building/radiation light invalidation recomputes affected cells immediately when those sources toggle/change.

## Patch Phases

### Phase 1 - Stop Known Wrong RGB Lighting

Goal: remove the most visible false behavior without broad renderer redesign.

Changes:

- Delete or retire `src/map/lighting.rs::apply_extra_light`.
- Remove the `lighting::apply_extra_light(...)` call from `src/app_init.rs`.
- Change `src/rules/art_data.rs` comments for `extra_light` from ambience to signed building body depth/Z adjustment.
- Keep parsing `ExtraLight=` as signed `i32` for now; do not rename public fields if that would churn call sites before the render-depth patch.
- Replace `test_extra_light_boost` with negative tests that prove RGB lighting is unchanged.

Files:

- `src/map/lighting.rs`
- `src/app_init.rs`
- `src/rules/art_data.rs`

Acceptance tests:

- `map_lighting_does_not_apply_extra_light_to_rgb_grid`
- `art_extra_light_preserves_signed_raw_depth_delta`
- `extra_light_negative_value_is_not_scaled_or_clamped_as_brightness`

Risk: low implementation risk, high screenshot correctness benefit for stock buildings using `ExtraLight=`.

### Phase 2 - Correct Ordinary Scenario Lighting Defaults And Formula

Goal: make static map ambience match binary arithmetic before touching renderer consumers.

Changes:

- Represent parsed ordinary `[Lighting]` fields in binary-like integer units or add helper conversion that is tested against binary scales.
- Default missing ordinary keys to `Ambient/R/G/B=1.00`, `Ground=0.20`, `Level=0.032`.
- Implement ordinary brightness as additive `ambient + level*z - ground`, not multiplicative ground darkening.
- Preserve separation of ordinary, Ion, Nuke/Lightning, and Dominator fields in comments/data shapes.
- Keep exact RGB normalization as a separate helper if the initial patch still returns compatibility `[f32;3]` tint.

Files:

- `src/map/lighting.rs`
- `src/app_init.rs` if call shape changes

Acceptance tests:

- `lighting_missing_ground_uses_binary_reset_default`
- `lighting_missing_level_uses_binary_reset_default`
- `lighting_ambient_rgb_keys_scale_by_100`
- `lighting_ground_level_keys_scale_by_250`
- `lighting_adds_level_and_subtracts_ground`
- `ordinary_lighting_ignores_dominator_fields_without_dynamic_flag`

Risk: high screenshot visibility. This changes every map whose `[Lighting]` omits `Ground`, and any map where `Ambient != 1.0` plus nonzero `Ground` previously hid the formula bug.

### Phase 3 - Fix Light Rules Parser Defaults And Malformed Float Semantics

Goal: make building light data match binary before rebuilding lamp math.

Changes:

- Change `ObjectType.light_visibility` default from `0` to `5000`.
- Keep `LightIntensity == 0.0` as the allocation/collection gate.
- Add a Westwood-float parser path for light fields and other `sscanf("%f")`-style keys where verified. It should parse the leading numeric prefix and stop at the first invalid trailing character, so `0,01` becomes `0.0`.
- Do not globally change every `get_f32` call unless the parser semantics are known safe for those keys; prefer a named helper if needed.
- Update tests that currently assume Rust full-string parse fallback for malformed floats.

Files:

- `src/rules/object_type.rs`
- `src/rules/ini_parser.rs`
- likely `src/rules/ini_parser_tests.rs`

Acceptance tests:

- `object_type_light_visibility_defaults_to_5000`
- `building_light_collection_gates_on_nonzero_intensity`
- `building_light_default_visibility_with_zero_intensity_collects_no_light`
- `building_light_default_visibility_with_nonzero_intensity_collects_light`
- `westwood_float_comma_decimal_parses_numeric_prefix_zero`
- `light_green_tint_0_comma_01_matches_gamemd_zero`

Risk: medium. Default visibility changes data parity but should not light default buildings because intensity remains zero. The parser helper can create broad fallout if applied to unrelated keys without evidence.

### Phase 4 - Replace Point-Light Math With Binary-Shaped Static Compute

Goal: make static lamp-post ambience credible for screenshot parity while still rebuilding only at map load.

Changes:

- Store point-light radius in leptons, not only cells.
- Use cell centers `cell*256 + 128`.
- Scan an inclusive square of `floor(radius / 256) + 1`, then filter by lepton center distance `<= LightVisibility`.
- Compute factor as `((radius - distance) * 1000) / radius`.
- Apply signed contribution division truncating toward zero for intensity and RGB tint fields.
- Sum contributions first; do not clamp each channel per source.
- Model DetailLevel gate: source contributes only when active and source threshold `2 <= DetailLevel`; default user detail is `2`.
- Preserve negative lamps.

Files:

- `src/map/lighting.rs`
- `src/app_init.rs` if point-light collection API changes
- optional new `src/map/light_profile.rs` only if Phase 5 is started here

Acceptance tests:

- `map_lighting_lepton_center_radius_edge_contributes_zero`
- `map_lighting_lepton_center_inside_radius_contributes_positive`
- `map_lighting_negative_lamp_truncates_toward_zero`
- `map_lighting_sums_sources_before_clamp`
- `map_lighting_lamp_suppressed_at_detail_0_enabled_at_2`
- `map_lighting_red_lamp_uses_tint_fields_as_signed_contributions`

Risk: high for lamp screenshots, but bounded to static map-load lighting if Phase 6 is deferred.

### Phase 5 - Introduce Cell Light Bundle And LightConvert-Style Profile Cache

Goal: stop treating map lighting as only RGB tint and give render paths the same categories binary draw code consumes.

Recommended data shape:

```rust
pub struct CellLight {
    pub profile: LightProfileId,
    pub scale_16_16: i32,
    pub aux: i16,
    pub top: i16,
    pub common: i16,
    pub bottom: i16,
    pub rgb_key: [i16; 3],
}

pub struct LightProfile {
    pub rgb_key: [i16; 3],
}
```

Naming can differ, but the ownership should not: profile key and scalar fields must remain separate.

Changes:

- Replace or wrap `LightingGrid = HashMap<(u16,u16), [f32;3]>` with a deterministic cell-light bundle grid.
- Seed a default full-bright profile for `(1000,1000,1000)`.
- Key the profile cache by normalized RGB triple only, not cell coordinate, height, raw f32, scalar brightness, or light-source identity.
- Normalize cache keys with binary clamp/quantization behavior: clamp each component to `0..1000`, then quantize by detail level (`128`, `64`, `32` unit masks for detail `0`, `1`, `2`).
- Preserve scalar fields `top/common/bottom` for branch-specific draw consumers.
- Expose a compatibility tint only as an adapter while render consumers are migrated; do not make it the authoritative model.

Files:

- `src/map/lighting.rs`
- `src/app_init.rs`
- `src/app.rs`
- `src/app_transitions.rs`
- `src/map/terrain.rs`
- `src/render/batch.rs` if instance format needs profile/scalar fields
- `src/render/palette_textures.rs` if profile-to-palette conversion is introduced

Acceptance tests:

- `lightconvert_default_profile_is_singleton`
- `lightconvert_profile_key_ignores_cell_coordinate`
- `lightconvert_profile_key_ignores_height_and_scalar_brightness`
- `lightconvert_profile_key_clamps_to_0_1000`
- `lightconvert_profile_key_quantizes_by_detail_level`
- `cell_light_bundle_preserves_top_common_bottom_scalars`

Risk: high architectural risk. This is the point where renderer APIs should change deliberately rather than stretching `[f32;3]` further.

### Phase 6 - Migrate Draw Consumers To Cell-Light Bundle

Goal: make terrain, overlays, terrain objects, Techno SHPs, and eligible anims consume the same lighting categories as binary paths.

Changes:

- TMP terrain/tile path should consume profile plus `common` scalar equivalent.
- Overlay paths should choose `top`, `common`, or `bottom` equivalents based on branch/type, not always one tint.
- Terrain objects should use `common` normally and `top` for the verified type-flag branch once that flag is represented.
- Building/infantry/unit SHP paths should use `common` where binary `TechnoClass_DrawSHP` does.
- Anim paths need metadata for cell-lighted, profile-selected, global/fixed, and fixed-1000 branches. Do not apply cell lighting to every anim.
- Keep bridges/shadows/railings that are known neutral separate until each path is verified.

Files:

- `src/map/terrain.rs`
- `src/app_instances/overlays.rs`
- `src/app_instances/shp.rs`
- `src/app_instances/units.rs`
- `src/app_instances/bridges.rs`
- `src/render/batch.rs`
- `src/render/tile_atlas.rs`
- `src/render/palette_textures.rs`

Acceptance tests:

- `map_lighting_tile_uses_profile_and_common_scalar`
- `map_lighting_overlay_can_use_top_common_or_bottom_scalar`
- `map_lighting_terrain_object_uses_cell_scalar`
- `map_lighting_techno_shp_uses_common_scalar`
- `anim_lighting_respects_cell_light_flags`
- `neutral_bridge_shadow_does_not_inherit_cell_light_without_verified_path`

Risk: very high screenshot risk and high regression risk. This should follow Phase 5 so every consumer migrates to a stable model.

### Phase 7 - Apply ExtraLight To Building Draw Depth

Goal: put `ExtraLight=` in its verified place after the building draw/depth surface is clear enough.

Changes:

- Add signed `extra_light` to the building body depth/Z-style calculation, using the cell `top`/`+0x10A` equivalent as the base where available.
- Do not apply it to building construction/gate-style branches that binary does not cover.
- Keep body and auxiliary body-art behavior separated if the Rust renderer gains those branches.
- Do not alter RGB profile, common scalar, or point-light fields from `ExtraLight=`.

Files:

- `src/app_instances/shp.rs`
- `src/rules/art_data.rs`
- possible depth helper in `src/app_instances/helpers.rs`
- possible `src/render/batch.rs` if depth key representation changes

Acceptance tests:

- `building_extra_light_adjusts_body_depth_not_rgb_lighting`
- `building_extra_light_negative_lowers_depth_key`
- `building_extra_light_positive_raises_depth_key`
- `building_extra_light_uses_cell_top_scalar_not_common_light_profile`
- `building_extra_light_not_applied_to_unverified_construction_branch`

Risk: medium/high. It affects sprite sort/depth for a small set of stock buildings but should be independent from RGB lighting once earlier phases are done.

### Phase 8 - Add Dynamic Immediate Dirty-Cell Recompute

Goal: update lighting when live sources change without full map rebuild.

Changes:

- Add runtime light-source state for building lamps and radiation sources.
- Implement immediate affected-cell recompute for standard verified callers: building online/offline, restore online effects, unlimbo/construction complete, owner change, sell, destruction/damage removal, radiation activate/AI update.
- Recompute affected cells inside the same radius scan as binary.
- Keep queued-mode infrastructure out of the normal path unless a live nonzero caller is later proven.
- If queued mode is implemented for completeness, commit all prepared records together; do not make per-record visible commits.

Files:

- likely `src/sim/` source-state ownership for building/radiation events
- `src/map/lighting.rs`
- `src/app.rs` / app-level bridge from sim updates to render lighting
- render invalidation/cache update surface from Phase 5

Acceptance tests:

- `destroying_lamp_recomputes_cells_inside_visibility_radius`
- `powering_lamp_off_recomputes_immediately`
- `powering_lamp_on_recomputes_immediately`
- `radiation_light_update_recomputes_immediately`
- `standard_building_light_toggle_does_not_use_queued_mode`
- `queued_light_commit_is_all_or_nothing_if_enabled`

Risk: high behavior risk because it crosses sim/render boundaries. Keep `sim/` independent from `render/`; sim should emit deterministic state/events, while app/render owns profile cache updates.

### Phase 9 - Spotlights As A Separate Feature

Goal: avoid accidentally implementing spotlight beams as radius ambience.

Changes:

- Parse `HasSpotlight=` and related spotlight rules only when implementing `BuildingLightClass`.
- Render direct beam/searchlight visuals separately from `LightSourceClass` ambience.
- Do not write spotlight effects into cell RGB, LightConvert profile, or lamp point-light collections.

Files:

- `src/rules/object_type.rs` or building type parser surface
- new render/app spotlight module as needed
- not `src/map/lighting.rs` except explicit negative tests

Acceptance tests:

- `has_spotlight_does_not_create_point_light_source`
- `has_spotlight_does_not_change_cell_light_profile`
- `spotlight_parser_is_separate_from_light_visibility_keys`

Risk: medium. Stock rules currently have no proven `HasSpotlight=` assignments, so this is lower priority than ordinary map lighting.

## File And Module Ownership

| Surface | Ownership | Expected role |
|---|---|---|
| `src/map/lighting.rs` | map/render data preparation | Scenario lighting parse/scale, cell-light compute, point-light accumulation, profile/cache-key helpers, static grid build. |
| `src/app_init.rs` | app orchestration | Build initial cell-light grid/profiles from map, rules, art, and resolved terrain; should not contain lighting formulas. |
| `src/app.rs` / `src/app_transitions.rs` | app state | Store the authoritative cell-light grid/profile cache and move it between load transitions. |
| `src/rules/object_type.rs` | rules parser | `LightVisibility`, `LightIntensity`, tint fields, future `HasSpotlight=` if not split elsewhere. |
| `src/rules/art_data.rs` | art parser | Preserve `ExtraLight=` as signed building draw-depth/Z delta; no ambience wording. |
| `src/rules/ini_parser.rs` | low-level parser helpers | Provide binary-compatible numeric helper where verified; avoid broad behavior changes without tests. |
| `src/map/terrain.rs` | terrain instance generation | Consume profile/common scalar for TMP terrain when renderer supports it. |
| `src/app_instances/overlays.rs` | overlay/terrain-object/effect instances | Consume branch-appropriate cell scalars; avoid one universal tint. |
| `src/app_instances/shp.rs` | building/infantry SHP instances | Consume common scalar for Techno-like SHPs; apply `ExtraLight=` only to building body depth/Z. |
| `src/app_instances/units.rs` | voxel/unit instances | Consume cell-light scalar/profile where unit render path needs it; keep non-light effects separate. |
| `src/render/batch.rs` | GPU instance contract | Evolve from RGB tint-only to profile/scalar inputs when Phase 5/6 is implemented. |
| `src/render/palette_textures.rs` | palette resources | Likely insertion point for profile-to-palette/convert resources if Rust models LightConvert on GPU. |
| `sim/` | deterministic gameplay | May own light-source existence/power/radiation state, but must not depend on render or palette types. |

## Acceptance Test Matrix

Minimum first patch set:

- `map_lighting_does_not_apply_extra_light_to_rgb_grid`
- `art_extra_light_preserves_signed_raw_depth_delta`
- `lighting_missing_ground_uses_binary_reset_default`
- `lighting_adds_level_and_subtracts_ground`
- `object_type_light_visibility_defaults_to_5000`
- `building_light_collection_gates_on_nonzero_intensity`
- `westwood_float_comma_decimal_parses_numeric_prefix_zero`

Static lamp parity set:

- `map_lighting_lepton_center_radius_edge_contributes_zero`
- `map_lighting_negative_lamp_truncates_toward_zero`
- `map_lighting_sums_sources_before_clamp`
- `map_lighting_lamp_suppressed_at_detail_0_enabled_at_2`
- `lightconvert_profile_key_quantizes_by_detail_level`

Renderer model set:

- `lightconvert_default_profile_is_singleton`
- `lightconvert_profile_key_ignores_cell_coordinate`
- `cell_light_bundle_preserves_top_common_bottom_scalars`
- `map_lighting_tile_uses_profile_and_common_scalar`
- `map_lighting_overlay_can_use_top_common_or_bottom_scalar`
- `map_lighting_techno_shp_uses_common_scalar`
- `anim_lighting_respects_cell_light_flags`

Depth and dynamic set:

- `building_extra_light_adjusts_body_depth_not_rgb_lighting`
- `building_extra_light_negative_lowers_depth_key`
- `destroying_lamp_recomputes_cells_inside_visibility_radius`
- `standard_building_light_toggle_does_not_use_queued_mode`
- `has_spotlight_does_not_create_point_light_source`

## Risk Ordering

1. Highest confidence and highest value: remove `ExtraLight=` from RGB lighting.
2. High confidence and broad screenshot impact: ordinary `[Lighting]` formula/defaults.
3. High confidence and bounded data impact: `LightVisibility=5000`, intensity gate, comma-prefix float parsing for verified light keys.
4. High screenshot impact but more math-sensitive: lepton-center point-light compute and RGB normalization.
5. Architectural turning point: cell-light bundle and LightConvert-style profile cache.
6. Broad renderer regression risk: migrating terrain/overlay/techno/anim consumers.
7. Isolated sort/depth risk: applying `ExtraLight=` to building body depth.
8. Cross-boundary runtime risk: dynamic immediate dirty recompute.
9. Separate low-stock-visibility feature: `BuildingLightClass` spotlights.

## Open Uncertainties

- Exact low-level `ConvertClass` and blitter palette table construction remains out of scope. The verified cache/profile key shape is enough for a renderer-facing abstraction, but not byte-exact palette table generation.
- Human-readable names for `Cell+0x104` and `Cell+0x108` are still not fully proven. Keep neutral/internal names until another report names them.
- Exact `Math__ftol` FPU rounding behavior was not independently audited. Existing reports verified surrounding integer formulas and truncation-sensitive paths, especially negative light contribution division.
- Special Ion/Nuke/Dominator transition timelines are separate systems. Ordinary map lighting must not consume those fields without active dynamic branch state.
- Some queued/cached/fogged draw paths are conditional. Standard YR does not use TS-style fog by default, but any later cached-object renderer should reuse the same cell-light bundle.
- Final insertion point for `ExtraLight=` depth depends on the building body renderer shape when Phase 7 is implemented.

## Do-Not-Implement Notes

- Do not implement map lamp ambience through Lightning Storm, weather, Tesla/EBOLT weapon visuals, or sound ambience.
- Do not use `ambient * (1.0 - ground)` as the parity formula.
- Do not use FinalAlert template `Ground=0.0` as the missing-key engine default.
- Do not let Ion, Nuke/Lightning, or Dominator fields tint ordinary maps unless their dynamic branch is active.
- Do not create or collect point lights merely because `LightVisibility` is nonzero; keep the `LightIntensity != 0` gate.
- Do not default `LightVisibility` to `0`; binary default is `5000`.
- Do not parse `0,01` as `0.01` for verified gamemd `sscanf("%f")` light keys; it parses as prefix `0.0`.
- Do not clamp after every light contribution; binary sums first, then normalizes/clamps.
- Do not ignore negative lamps or floor their negative contributions; binary signed division truncates toward zero.
- Do not key LightConvert/profile cache by cell coordinate, height, raw f32, scalar brightness, or light-source identity.
- Do not collapse `top`, `common`, and `bottom` scalar fields into one universal tint once renderer parity work begins.
- Do not apply `ExtraLight=` to RGB lighting, radius lights, `LightSourceClass`, `LightConvertClass`, or cell `+0x10C`.
- Do not divide `ExtraLight=` by `1000`; binary uses raw signed 16-bit addition to the depth/Z-style draw argument.
- Do not merge `BuildingClass+0x600` spotlight beams with `BuildingClass+0x614` lamp/radius ambience.
- Do not introduce `sim/` dependencies on `render/`, palette, UI, or audio while adding dynamic lighting updates.

## Recommended First Implementation Ticket

Implement Phases 1-3 as one focused patch if the diff stays small:

1. Remove `ExtraLight=` from RGB lighting and fix comments/tests.
2. Correct ordinary `[Lighting]` defaults and additive formula.
3. Correct `LightVisibility` default and add verified light-key float parser tests.

Stop before LightConvert/profile work. That keeps the first patch high-confidence, testable, and visible while leaving the renderer architecture change for a deliberate second pass.
