# Map Lighting — Final System Model Synthesis

Date: 2026-05-22
System: ordinary map lighting, lamp/radius ambience, LightConvert profiles, draw consumers, `ExtraLight=`, and spotlights.
Non-scope: Lightning Storm damage/weather lifecycle, EBOLT/Tesla weapon visuals, sound ambience, full ConvertClass palette-table internals.
Output type: model-synthesis with bounded remaining research.

## Current Model

YR map lighting is not one feature. It is a render-facing cell-light system fed by scenario ambience and optional runtime light sources.

1. **Scenario ambience** comes from map `[Lighting]` keys `Ambient`, `Red`, `Green`, `Blue`, `Ground`, and `Level`. Fresh map load resets defaults before reading the map: `Ambient/R/G/B=1.00`, `Ground=0.20`, `Level=0.032`. Ordinary brightness is additive: `Ambient + Level * cell_level - Ground`, not `Ambient * (1 - Ground)`.
2. **Lamp/radius lights** are `LightSourceClass` instances, usually from building type keys `LightVisibility`, `LightIntensity`, `LightRedTint`, `LightGreenTint`, and `LightBlueTint`. Building `+0x614` owns this pointer. Allocation/collection is gated by nonzero `LightIntensity`, not `LightVisibility`.
3. **LightConvert** is a render-facing profile/cache. Cells store a profile pointer at `CellClass+0x34` plus scalar light fields at `+0x104..+0x114`. The cache key is the normalized/quantized RGB triple, not the cell coordinate.
4. **Draw consumers** use different cell fields. Terrain/TMP, overlays, terrain objects, Techno SHPs, anims, and building body drawing do not all consume the same one RGB tint.
5. **`ExtraLight=` is not light.** It is art/image data stored as signed `BuildingTypeClass+0x1548`, added to signed `Cell+0x10A` in `BuildingClass_DrawBody` as a depth/Z-style draw adjustment.
6. **`BuildingLightClass` spotlights are separate.** Building `+0x600` is `HasSpotlight=` direct beam/searchlight rendering. It does not write cell RGB ambience or LightConvert values.

## Claim Table

| Claim | Best evidence | Status | Confidence | Active in YR | Safe? |
|---|---|---|---|---|---|
| Ordinary map ambience uses `[Lighting]` and additive `Ambient + Level*z - Ground` | `SCENARIO_LIGHTING_FIELDS_00689E90_GHIDRA_REPORT.md`; `SCENARIO_LIGHTING_DEFAULT_RESET_PATH_GHIDRA_REPORT.md` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| Missing ordinary lighting keys preserve `1.00/1.00/1.00/1.00/0.20/0.032` | `SCENARIO_LIGHTING_DEFAULT_RESET_PATH_GHIDRA_REPORT.md` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| Lamp keys are read from building rules/self section | `BUILDINGTYPE_LIGHT_KEYS_READINI_CONSTANTS_GHIDRA_REPORT.md` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| `LightVisibility` default is direct int `5000` | same; `BUILDINGTYPECLASS_CTOR_DEFAULTS.md` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| `LightIntensity` and tint keys store `ftol(value*1000+0.1)` | `BUILDINGTYPE_LIGHT_KEYS_READINI_CONSTANTS_GHIDRA_REPORT.md` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| Missing tint keys preserve raw ctor `1000000`; explicit `1.0` stores `1000` | same | confirmed | high | yes | IMPLEMENTATION_SAFE, but note raw-scale oddity |
| `LightGreenTint=0,01` stores `0` | same; `LIGHT_RULES_ART_PARSER_DEFAULTS_GHIDRA_REPORT.md` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| Lamp falloff uses lepton-center integer math, inclusive radius, signed contribution | `MAP_LIGHTING_CELL_COMPUTE_00484180_GHIDRA_REPORT.md` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| Standard building/radiation light invalidation is immediate, not queued | `LIGHTSOURCE_QUEUED_MODE_CALLER_CENSUS_GHIDRA_REPORT.md` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| LightConvert cache key is normalized RGB triple | `MAP_LIGHTCONVERT_CACHE_00483E30_00544E70_GHIDRA_REPORT.md`; `LIGHTCONVERT_NORMALIZE_005558E0_00555AC0_GHIDRA_REPORT.md` | confirmed | high | yes | IMPLEMENTATION_SAFE for profile shape |
| `ExtraLight=` is signed draw-depth/Z delta, not RGB ambience | `BUILDINGTYPE_EXTRALIGHT_0X1548_CONSUMER_GHIDRA_REPORT.md`; `EXTRALIGHT_DRAWBODY_Z_DELTA_DETAILS_GHIDRA_REPORT.md` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| `BuildingLightClass` spotlights write cell ambience | `BUILDINGLIGHTCLASS_SPOTLIGHT_PATH_GHIDRA_REPORT.md` | contradicted | high | conditional | DOC_PATCH_READY |
| Exact post-load caller that rehydrates `+0x614` | `LIGHTSOURCE_LIFECYCLE_POWER_DAMAGE_SAVELOAD_GHIDRA_REPORT.md` | unknown | medium | likely | NEEDS_REINVESTIGATE |
| Low-level spotlight beam rasterization | `BUILDINGLIGHTCLASS_SPOTLIGHT_PATH_GHIDRA_REPORT.md` | unknown | medium | conditional | NEEDS_REINVESTIGATE |

## Implementation-Safe Facts

- Remove `ExtraLight=` from RGB map lighting. It belongs in building body depth/Z handling as raw signed addition to the `Cell+0x10A` equivalent.
- Use binary missing-key defaults for ordinary map lighting: `Ambient/R/G/B=1.00`, `Ground=0.20`, `Level=0.032`.
- Implement ordinary map brightness as additive `Ambient + Level*z - Ground`. Keep Ion/Nuke/Dominator fields separate unless the dynamic branch is active.
- Default `LightVisibility` to `5000`, but collect/create building light sources only when `LightIntensity != 0`.
- Parse verified light-key floats with Westwood/`sscanf("%f")` prefix semantics. Stock `0,01` becomes `0.0`, not `0.01` and not fallback default.
- Treat lamp radius in leptons. Use cell centers `cell*256 + 128`, inclusive radius checks, signed contributions, truncation toward zero, and sum-before-normalize behavior.
- Model a cell-light bundle rather than only `[f32;3]`: profile identity plus scalar fields corresponding to top/common/bottom draw paths.
- Use a LightConvert-style profile cache keyed by normalized/quantized RGB triple. Do not key by cell coordinate, height, scalar brightness, or light-source identity.
- For standard YR building/radiation lighting changes, implement immediate affected-cell recompute. Keep queued mode latent unless a live nonzero caller is later proven.
- Keep `BuildingLightClass` spotlights out of lamp/radius ambience. Parse/render them later as direct beam visuals if needed.

## Doc-Patch-Ready Facts

- Docs saying `+0x614` is default/all-building ambient light are stale. The checked gate is nonzero `LightIntensity`.
- Docs or comments describing `ExtraLight=` as ambience or brightness are wrong. It is a signed building-body depth/Z delta.
- Comments claiming Rust point-light math exactly matches YR should be softened or replaced; current Rust has only the broad linear falloff idea.
- Any doc using FinalAlert template `Ground=0.0` as missing-key engine default should be corrected to binary reset `Ground=0.20`.
- Any doc merging `+0x600 BuildingLightClass` with `+0x614 LightSourceClass` should be corrected.

## Stale Or Superseded Claims

- `MAP_LIGHTING_AND_LIGHT_POSTS_SYSTEM_MODEL_SYNTHESIS.md` marked exact LightSource propagation as needing more work. Superseded for the main static lamp path by the two re-swarms; remaining gaps are narrow.
- `BUILDINGCLASS_MASTER_GHIDRA_REPORT.md` and older verification prose describe `+0x614` as "on all buildings" or "default ambient light". Superseded by Unlimbo and lifecycle reports.
- Current Rust comments in `src/map/lighting.rs` around `ExtraLight` and exact point-light parity are superseded by the new reports.

## Cross-Doc Conflicts

- "Ambient" is overloaded. Use these names:
  - scenario ambience: map `[Lighting]` fields;
  - radius light: `LightSourceClass` at building `+0x614`;
  - spotlight/searchlight: `BuildingLightClass` at building `+0x600`;
  - art depth delta: `ExtraLight=`.
- The raw tint default `1000000` conflicts with intuitive `1.0 == 1000` scaling. This is verified; implementation should preserve observed behavior or explicitly document any normalization layer that makes the two equivalent downstream.

## Needs Re-Investigation

- `/re-investigate BuildingClass LightSource post-load rehydrate caller`  
  Load zeroes `+0x614`; lifecycle is otherwise known. Find the exact outer post-load path that recreates/enables runtime light sources.
- `/re-investigate BuildingLightClass beam rasterization and ProcessCellAction 0x23`  
  Only needed when implementing `HasSpotlight=` beams. It does not block ordinary map ambience or lamp/radius lights.
- `/re-investigate ConvertClass low-level palette table generation for LightConvert`  
  Only needed for byte-exact palette-table work. Current evidence is enough for a renderer-facing profile/cache abstraction.

## Do-Not-Implement Notes

- Do not implement map lighting through Lightning Storm/weather logic.
- Do not use `ambient * (1.0 - ground)` as parity behavior.
- Do not apply `ExtraLight / 1000.0` to RGB cells.
- Do not create lights from `LightVisibility` alone.
- Do not parse verified light-key `0,01` as `0.01`.
- Do not clamp per light contribution; sum first, then normalize/clamp.
- Do not collapse all draw consumers to one universal tint once renderer parity work begins.
- Do not serialize `LightSourceClass` handles; runtime light state should be rebuilt from building state/type data.
- Do not let `sim/` depend on render or palette types when adding dynamic light invalidation.

## Source Ledger

- Reconciliations: `MAP_LIGHTING_RE_SWARM_RECONCILIATION_2026_05_22.md`, `MAP_LIGHTING_RE_SWARM_ROUND2_RECONCILIATION_2026_05_22.md`.
- Implementation handoff: `MAP_LIGHTING_IMPLEMENTATION_SPEC.md`, `MAP_LIGHTING_RUST_HANDOFF_AUDIT.md`.
- Ghidra reports: `SCENARIO_LIGHTING_FIELDS_00689E90_GHIDRA_REPORT.md`, `SCENARIO_LIGHTING_DEFAULT_RESET_PATH_GHIDRA_REPORT.md`, `MAP_LIGHTING_CELL_COMPUTE_00484180_GHIDRA_REPORT.md`, `MAP_LIGHTCONVERT_CACHE_00483E30_00544E70_GHIDRA_REPORT.md`, `LIGHTCONVERT_NORMALIZE_005558E0_00555AC0_GHIDRA_REPORT.md`, `LIGHTSOURCE_DIRTY_SCHEDULING_00554AF0_00554D50_GHIDRA_REPORT.md`, `LIGHTSOURCE_QUEUED_MODE_CALLER_CENSUS_GHIDRA_REPORT.md`, `LIGHTSOURCE_LIFECYCLE_POWER_DAMAGE_SAVELOAD_GHIDRA_REPORT.md`, `BUILDINGTYPE_LIGHT_KEYS_READINI_CONSTANTS_GHIDRA_REPORT.md`, `BUILDINGTYPE_EXTRALIGHT_0X1548_CONSUMER_GHIDRA_REPORT.md`, `EXTRALIGHT_DRAWBODY_Z_DELTA_DETAILS_GHIDRA_REPORT.md`, `BUILDINGLIGHTCLASS_SPOTLIGHT_PATH_GHIDRA_REPORT.md`, `LIGHTING_DRAW_CONSUMERS_CELL_FIELDS_GHIDRA_REPORT.md`, `LIGHT_RULES_ART_PARSER_DEFAULTS_GHIDRA_REPORT.md`.
- Supporting older docs: `BUILDINGTYPECLASS_CTOR_DEFAULTS.md`, `BUILDINGTYPECLASS_FIELDS.csv`, `BUILDINGCLASS_FIELD_VERIFICATION_ROUND_2.md`, `CELL_COMPUTE_ZADJUST_FORMULA_GHIDRA_REPORT.md`.
- INI/Rust surfaces: `ini/rulesmd.ini`, `ini/artmd.ini`, `src/map/lighting.rs`, `src/app_init.rs`, `src/rules/object_type.rs`, `src/rules/art_data.rs`, `src/rules/ini_parser.rs`.

## Classification

Implementation-safe for ordinary map ambience, lamp/radius lights, parser/default fixes, `ExtraLight=` removal from RGB lighting, and the render-facing LightConvert profile shape. Investigation-blocked only for exact post-load `+0x614` rehydration, low-level spotlight beams, and byte-exact ConvertClass palette-table internals.
