# Map Lighting Re-Swarm Reconciliation

Date: 2026-05-22
Scope: map `[Lighting]`, lamp/radius lights, LightConvert cache, dirty recompute, `ExtraLight=`, spotlights, parser defaults, and Rust handoff.
Result: 6 COMPLETE, 2 PARTIAL. No Rust or existing research docs were patched.

## Reports

| Slot | Report | Status | Result |
|---|---|---|---|
| 1 | `MAP_LIGHTING_CELL_COMPUTE_00484180_GHIDRA_REPORT.md` | COMPLETE | Exact cell-light compute path verified. |
| 2 | `MAP_LIGHTCONVERT_CACHE_00483E30_00544E70_GHIDRA_REPORT.md` | COMPLETE | LightConvert cache/refcount/profile shape verified. |
| 3 | `LIGHTSOURCE_DIRTY_SCHEDULING_00554AF0_00554D50_GHIDRA_REPORT.md` | PARTIAL | Dirty recompute path mostly verified; queued-mode live caller remains unproven. |
| 4 | `BUILDINGTYPE_EXTRALIGHT_0X1548_CONSUMER_GHIDRA_REPORT.md` | COMPLETE | `ExtraLight=` is not lighting; it is draw-depth/Z adjustment. |
| 5 | `BUILDINGLIGHTCLASS_SPOTLIGHT_PATH_GHIDRA_REPORT.md` | COMPLETE | Spotlights are direct beam rendering, separate from lamp ambience. |
| 6 | `SCENARIO_LIGHTING_FIELDS_00689E90_GHIDRA_REPORT.md` | COMPLETE | Map ambience parser/scaling/formula verified. |
| 7 | `LIGHT_RULES_ART_PARSER_DEFAULTS_GHIDRA_REPORT.md` | PARTIAL | Defaults and malformed-float behavior verified; exact per-key multiply constants need xref slice. |
| 8 | `MAP_LIGHTING_RUST_HANDOFF_AUDIT.md` | COMPLETE | Current Rust deltas and patch order identified. |

## Reconciled Findings

1. **Current Rust map ambience formula is wrong for parity.** Binary ordinary map lighting is additive: `Ambient + Level * cell_level - Ground`, with binary scaling. Rust currently computes `ambient * (1.0 - ground) + level * z`.
2. **Lamp falloff is real but Rust is only approximate.** Binary uses integer lepton-center math: `cell*256 + 128`, inclusive radius check, `((radius - distance) * 1000) / radius`, truncation toward zero, detail gating, and post-sum normalization through the LightConvert pipeline.
3. **LightConvert is a render-facing RGB profile cache.** `CellClass+0x34` stores `LightConvertClass*`; cache lookup is keyed by normalized RGB triple only, not cell coordinate. Refcount is at `LightConvert+0x194`.
4. **`ExtraLight=` must be removed from map RGB lighting.** It is a signed 16-bit `BuildingTypeClass+0x1548` draw-depth/Z adjustment consumed by `BuildingClass_DrawBody`, not an ambience key.
5. **`LightVisibility` default is 5000.** Rust currently defaults it to 0. This does not light every building because the allocation/collection gate remains nonzero `LightIntensity`.
6. **Malformed stock float values matter.** Stock `LightGreenTint=0,01` parses as `0.0` in gamemd through `sscanf("%f")`; Rust `parse::<f32>()` fails and currently falls back to default.
7. **Dynamic light invalidation is missing.** Building online/offline/destruction and radiation paths can recompute affected cells; current Rust bakes a startup grid.
8. **`BuildingLightClass` spotlights are separate.** `BuildingClass+0x600` is `HasSpotlight=` beam rendering; it does not write LightConvert/per-cell RGB ambience and should not be merged with lamp posts.

## Cross-Report Consistency

- Slots 1, 2, and 6 agree on the rendering pipeline: scenario fields and light sources feed scalar/RGB cell fields, then `LightConvert` provides render-facing conversion.
- Slots 4, 7, and 8 agree that `ExtraLight=` is currently mis-modeled in Rust.
- Slots 1, 3, and 5 agree that lamp ambience (`+0x614 LightSourceClass`) is separate from spotlight beams (`+0x600 BuildingLightClass`).
- No report supported treating map lamp posts as Lightning Storm/weather behavior.

## Implementation Handoff Order

1. **Fix the wrong `ExtraLight=` behavior first.**
   - Remove `apply_extra_light` from RGB lighting.
   - Preserve parsed `ExtraLight` for later building draw-depth/Z integration.
   - Test: `map_lighting_does_not_apply_art_extra_light_to_rgb_tint`.

2. **Correct map `[Lighting]` formula and defaults.**
   - Implement binary additive formula and scaling in fixed-point or integer domain.
   - Use ScenarioClass-style defaults: Ambient/R/G/B 100, Ground 50, Level 8 internally.
   - Test: `lighting_ground_subtracts_after_ambient_not_multiply`.

3. **Correct light parser defaults and malformed float handling.**
   - Default `LightVisibility` to 5000.
   - Keep `LightIntensity != 0` as collection/allocation gate.
   - Make RA2 float parsing treat comma-decimal stock values like gamemd, where `0,01` becomes `0.0`.
   - Tests: `building_light_visibility_defaults_to_5000`, `ra2_float_comma_decimal_parses_prefix_zero`.

4. **Replace approximate point-light math with binary-shaped math.**
   - Use lepton-center coordinates, inclusive radius filter, integer/truncating contribution, negative lamp support, detail gate, and post-sum clamp/normalize order.
   - Tests: `lamp_light_uses_lepton_center_distance`, `negative_lamp_darkens_with_truncation_toward_zero`.

5. **Add a render-facing LightConvert profile/cache abstraction.**
   - Key by normalized RGB triple, not cell coordinate.
   - Track enough profile identity to match draw ordering/palette choices even if Rust does not need raw refcounts internally.
   - Test: `lightconvert_profile_reuses_same_rgb_triple_for_many_cells`.

6. **Add dynamic dirty-cell recompute after the static path is correct.**
   - Building light toggle/destruction and radiation should recompute affected cells.
   - Keep queued all-or-nothing commit behavior for any implemented queued path.
   - Test: `destroying_lamp_recomputes_cells_inside_visibility_radius`.

7. **Implement `BuildingLightClass` spotlights later as direct beam visuals.**
   - Parse `HasSpotlight=`.
   - Do not feed it into RGB ambience.
   - Test: `has_spotlight_does_not_create_point_light_source`.

## Remaining Research

- Bounded xref/assembly slice for exact per-key multiply constants in the huge BuildingType reader.
- Prove whether any standard YR caller passes nonzero queued mode into LightSource dirty scheduling.
- Low-level spotlight beam rasterization and `ProcessCellAction(0x23)` behavior, only needed when implementing spotlights.

## Bottom Line

Map lighting deserves follow-up implementation work before screenshot parity. The swarm found concrete mismatches in current Rust, especially `ExtraLight=`, map ambience formula, parser defaults, malformed float parsing, and LightConvert modeling.
