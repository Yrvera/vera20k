# Map Lighting Re-Swarm Round 2 Reconciliation

Date: 2026-05-22
Scope: remaining map-lighting implementation blockers after the first swarm.
Result: 7 COMPLETE, 1 PARTIAL. No Rust or existing research docs were patched.

## Reports

| Slot | Report | Status | Result |
|---|---|---|---|
| 1 | `BUILDINGTYPE_LIGHT_KEYS_READINI_CONSTANTS_GHIDRA_REPORT.md` | COMPLETE | Exact BuildingType light-key reader constants and ownership verified. |
| 2 | `LIGHTSOURCE_QUEUED_MODE_CALLER_CENSUS_GHIDRA_REPORT.md` | COMPLETE | No active standard YR nonzero queued-mode caller found. |
| 3 | `LIGHTCONVERT_NORMALIZE_005558E0_00555AC0_GHIDRA_REPORT.md` | COMPLETE | Light normalization and detail-level RGB quantization verified. |
| 4 | `LIGHTING_DRAW_CONSUMERS_CELL_FIELDS_GHIDRA_REPORT.md` | COMPLETE | Draw consumers of cell light fields mapped. |
| 5 | `SCENARIO_LIGHTING_DEFAULT_RESET_PATH_GHIDRA_REPORT.md` | COMPLETE | Scenario lighting reset/default path verified. |
| 6 | `EXTRALIGHT_DRAWBODY_Z_DELTA_DETAILS_GHIDRA_REPORT.md` | COMPLETE | `ExtraLight=` exact DrawBody Z/depth use verified. |
| 7 | `LIGHTSOURCE_LIFECYCLE_POWER_DAMAGE_SAVELOAD_GHIDRA_REPORT.md` | PARTIAL | Building LightSource lifecycle verified except exact outer post-load rehydrate caller. |
| 8 | `MAP_LIGHTING_IMPLEMENTATION_SPEC.md` | COMPLETE | Non-editing Rust implementation spec written. |

## New Facts Since Round 1

1. **BuildingType lamp keys are rules/self-section keys.** `LightVisibility`, `LightIntensity`, and RGB tint keys are read from the building's rules section, not art/image fallback.
2. **`ExtraLight=` is an art/image-section key.** It is read from the image/art section and stored at `BuildingTypeClass+0x1548` as signed 16-bit.
3. **Light key scaling is exact enough for implementation.**
   - `LightVisibility=` is direct int, default `5000`.
   - `LightIntensity` and RGB tint keys store `ftol(value * 1000.0 + 0.1)`.
   - Missing tint keys preserve constructor raw `1000000`.
   - Explicit `LightRedTint=1.0` stores `1000`.
   - Stock `LightGreenTint=0,01` parses as leading `0.0`, so it stores `0`.
4. **Scenario lighting defaults are confirmed by reset path.** Fresh map load resets before `[Lighting]` read. Missing ordinary keys preserve `Ambient/R/G/B=1.00`, `Ground=0.20`, `Level=0.032`.
5. **Queued LightSource mode appears latent for standard YR.** No drained active building/radiation caller passes nonzero mode. Implement standard invalidation as immediate recompute; leave queued mode as future infrastructure unless a live caller is later proven.
6. **LightConvert has two separate stages.**
   - `0x005558E0`: clamp/normalize light values and carry overbright through a 16.16 scale.
   - `0x00555AC0`: quantize normalized RGB by detail-level mask, with default `(1000,1000,1000)` bypassing this helper in the default cache path.
7. **Draw consumers require more than a single RGB grid.**
   - Terrain/TMP draw consumes `Cell+0x34` and `Cell+0x10C`.
   - Overlays, terrain objects, Techno SHPs, and animations use distinct cell-light branches.
   - `+0x110/+0x112/+0x114` mirror the normalized RGB cache key.
8. **`ExtraLight=` exact application is signed cell depth adjustment.** Building body draw computes `signed(Cell+0x10A) + signed(BuildingType+0x1548)`. Main body uses layer `2`, sort flag `1`; auxiliary body-art branches use layer `0`; a special gate/construction-style branch uses `Cell+0x10A` without `ExtraLight`.
9. **Building LightSource lifecycle is mostly nailed down.** `+0x614` starts inactive, is enabled on Unlimbo/construction complete, toggled on power/capture transitions, disabled on sell/death, deleted/zeroed in destructor, and zeroed on load. Runtime light state should be rebuilt after load, not serialized as durable handle.

## Cross-Report Corrections

- The round-1 implementation note "default tints 1.0" is too simple. In binary, missing tint keys preserve raw constructor `1000000`, while explicit `1.0` stores `1000`. Rust should preserve the observed downstream behavior, not blindly treat missing and explicit `1.0` as identical until the LightSource constructor/consumer scale contract is represented.
- `ExtraLight` should be removed from `src/map/lighting.rs` RGB math immediately, but the correct positive/negative visual effect belongs in building-body draw depth, not in a generic entity light pass.
- Dynamic dirty recompute can be implemented as immediate for standard YR lamp/radiation paths. The queued pipeline should not block initial parity work.

## Updated Patch Order

1. **Remove `ExtraLight` from RGB map lighting.**
   - Delete/disable `apply_extra_light` from lighting-grid construction.
   - Keep parsed art `ExtraLight` data for building-body draw depth.
   - Tests: `map_lighting_ignores_art_extra_light_rgb`, `building_body_extra_light_adds_signed_cell_10a_delta`.

2. **Represent cell lighting as integer profile data, with a temporary RGB adapter only where needed.**
   - Track fields corresponding to `Cell+0x10A`, `+0x10C`, and normalized RGB key.
   - Add LightConvert-style profile cache keyed by quantized RGB triple.
   - Tests: `lightconvert_default_rgb_uses_single_profile`, `lightconvert_quantizes_by_detail_level`.

3. **Fix `[Lighting]` defaults and formula.**
   - Defaults: Ambient/R/G/B `1.00`, Ground `0.20`, Level `0.032`.
   - Formula: additive `Ambient + Level*z - Ground`, then binary clamp/normalize behavior.
   - Tests: `lighting_missing_keys_use_binary_reset_defaults`, `lighting_ground_is_subtracted_not_multiplied`.

4. **Fix parser/default semantics.**
   - `LightVisibility` default `5000`.
   - `LightIntensity != 0` remains light-source gate.
   - Implement RA2 float prefix parsing so `0,01` becomes `0.0`.
   - Preserve missing vs explicit tint behavior if modeling raw BuildingType fields.
   - Tests: `light_visibility_missing_defaults_5000`, `ra2_float_comma_decimal_parses_prefix_zero`, `missing_light_tint_preserves_raw_ctor_default`.

5. **Replace approximate lamp math.**
   - Use lepton-center integer distance, inclusive radius, truncation toward zero, negative lamp support, detail gates, and post-sum normalization.
   - Tests: `lamp_falloff_uses_cell_center_leptons`, `negative_lamp_truncates_toward_zero`.

6. **Add immediate dynamic invalidation hooks.**
   - Hook building placement/construction complete, online/offline/capture, sell/death/destructor, and load rehydration.
   - Do not serialize runtime LightSource handles.
   - Tests: `lamp_power_off_recomputes_radius_immediately`, `loaded_lamp_rebuilds_runtime_light_source`.

7. **Defer spotlights and queued-mode infrastructure.**
   - `BuildingLightClass` spotlights are direct beam visuals, not map ambience.
   - Nonzero queued LightSource mode has no proven standard caller.

## Remaining Research

- Exact outer post-load caller that rehydrates `BuildingClass+0x614` after load zeroing.
- Low-level spotlight beam rasterization and `ProcessCellAction(0x23)` if/when implementing `HasSpotlight=`.
- Whether the raw missing-tint `1000000` value has any mod-visible edge case beyond the checked LightSource constructor/consumer path.

## Bottom Line

Round 2 converts map lighting from "research-worthy" to "implementation-ready for the main ambience/lamp path." The remaining uncertainty is narrow and does not block the first patch series: remove `ExtraLight` from RGB lighting, fix map ambience formula/defaults, fix parser semantics, and introduce a render-facing LightConvert/profile model.
