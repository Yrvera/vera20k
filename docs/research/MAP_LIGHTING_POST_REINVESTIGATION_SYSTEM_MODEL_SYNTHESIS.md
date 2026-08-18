# Map Lighting Post-Reinvestigation System Model Synthesis

Date: 2026-05-22
System: ordinary map lighting, lamp/light-post ambience, `LightSourceClass`
lifecycle, LightConvert render profiles, `ExtraLight=`, and Rust implementation
safety after the post-load rehydrate reinvestigation.
Output type: model-synthesis.

Non-scope: Lightning Storm/weather logic, EBOLT/Tesla visuals, audio ambience,
low-level `BuildingLightClass` spotlight beam rasterization, and byte-exact
ConvertClass palette-table generation.

## Current Model

Ordinary YR map lighting is a render-facing cell-light system, not the Lightning
Storm system and not ambient sound. It has three separate concepts that Rust
must keep apart:

1. Scenario ambience from map `[Lighting]`: `Ambient`, `Red`, `Green`, `Blue`,
   `Ground`, and `Level`. Missing keys preserve binary reset defaults
   `Ambient/R/G/B=1.00`, `Ground=0.20`, `Level=0.032`. Brightness is additive:
   `Ambient + Level * cell_level - Ground`, not `Ambient * (1 - Ground)`.
2. Lamp/light-post radius ambience from `LightSourceClass`, stored on buildings
   as `BuildingClass+0x614`. Building allocation is gated by nonzero
   `LightIntensity`, not by default/nonzero `LightVisibility` alone.
3. Render conversion through cell scalar fields and a LightConvert-style profile
   cache. `Cell+0x34` is the profile pointer; `Cell+0x104..+0x114` hold scalar
   and normalized RGB-key fields. Different draw consumers select different
   scalar fields.

The post-load reinvestigation changes the lifecycle model: `BuildingClass::Load`
unconditionally zeroes `+0x614`, and no general post-load lazy rehydrate caller
was verified. For Rust, the safe model is still: do not serialize runtime light
handles; rebuild app/render light state explicitly from durable building state
after loading a snapshot.

## Claim Table

| Claim | Best evidence | Status | Confidence | Active in YR | Safe? |
|---|---|---|---|---|---|
| `[Lighting]` ordinary brightness is additive, with `Ground=0.20` missing-key reset | `SCENARIO_LIGHTING_*` reports | confirmed | high | yes | IMPLEMENTATION_SAFE |
| Current Rust `ambient * (1-ground)` and `Ground=0.0` default are wrong | `src/map/lighting.rs`; scenario reports | confirmed | high | yes | IMPLEMENTATION_SAFE |
| Building lamp allocation/collection gate is `LightIntensity != 0` | `BUILDINGTYPE_LIGHT_KEYS_*`; Ghidra spot-check `0x00554760` callers | confirmed | high | yes | IMPLEMENTATION_SAFE |
| `LightVisibility` binary default is `5000` | `BUILDINGTYPE_LIGHT_KEYS_*`; `rules.ini` comment and stock lamp entries | confirmed | high | yes | IMPLEMENTATION_SAFE |
| Stock `LightGreenTint=0,01` parses/stores as `0` | `BUILDINGTYPE_LIGHT_KEYS_*`; `rules.ini` occurrences | confirmed | high | yes | IMPLEMENTATION_SAFE for light keys |
| Lamp falloff uses lepton-center integer math and sum-before-normalize | `MAP_LIGHTING_CELL_COMPUTE_00484180_GHIDRA_REPORT.md` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| LightConvert cache key is normalized RGB triple, not cell coordinate | `MAP_LIGHTCONVERT_CACHE_*`; `LIGHTCONVERT_NORMALIZE_*` | confirmed | high | yes | IMPLEMENTATION_SAFE for profile shape |
| `ExtraLight=` is signed building body depth/Z adjustment, not RGB light | `BUILDINGTYPE_EXTRALIGHT_*`; `EXTRALIGHT_DRAWBODY_*` | confirmed | high | yes | IMPLEMENTATION_SAFE |
| `BuildingLightClass+0x600` spotlights are separate from `LightSourceClass+0x614` | `BUILDINGLIGHTCLASS_SPOTLIGHT_PATH_GHIDRA_REPORT.md` | confirmed | high | conditional | IMPLEMENTATION_SAFE separation |
| `BuildingClass::Load` preserves/fixes up `+0x614` | `BUILDING_LIGHTSOURCE_POST_LOAD_REHYDRATE_GHIDRA_REPORT.md`; spot-check `0x00454174` | contradicted | high | yes | DOC_PATCH_READY |
| A general first-post-load tick caller rehydrates `+0x614` | post-load reinvestigation | unverified/stale | medium | unknown | DOC_PATCH_READY correction; runtime trace optional |
| Rust should serialize a LightSource handle | load zeroing and lifecycle reports | contradicted | high | yes | DO_NOT_IMPLEMENT |

## Spot Checks

- Ghidra call graph for `LightSourceClass__Constructor @ 0x00554760` shows only
  building callers `BuildingClass::Unlimbo` and `BuildingClass::OnConstructionComplete`,
  plus radiation activation. No third building allocator appeared.
- Ghidra assembly context confirms `BuildingClass::Load` writes
  `MOV [EDI+0x614],0` at `0x00454174`.
- Ghidra assembly context confirms `OnConstructionComplete` stores the constructor
  result to `[EBP+0x614]` at `0x00446759`, then immediately calls `0x00554A60(0)`.
- Ghidra assembly context confirms `Mission_Construction` calls
  `vtable+0x4DC` with pushed arg `0` only in the construction-complete tail
  (`0x00449AD0..0x00449AD4`), not as a load driver.
- INI spot-check: stock `ini/rules.ini` contains `LightGreenTint=0,01` at
  multiple lamp entries, and the rules comment states `LightVisibility` default
  is `5000`.

## Implementation-Safe Facts

- First patch should remove `ExtraLight=` from RGB map lighting and keep its raw
  signed value for building-body draw-depth/Z integration.
- Implement ordinary map ambience defaults and formula before attempting byte
  exact lamp screenshots.
- Set `LightVisibility` default to `5000`, but keep the actual light creation
  gate as `LightIntensity != 0`.
- Add a light-key float parser matching Westwood prefix parsing; do not globally
  change every float key until each family is verified.
- Replace provisional point-light math with lepton-center integer falloff:
  cell center `cell*256+128`, inclusive radius, signed contribution, truncation
  toward zero, sum first, normalize/clamp after.
- Introduce a cell-light bundle and LightConvert-style profile cache keyed by
  normalized/quantized RGB triple. Keep top/common/bottom scalar fields separate.
- After Rust snapshot load, rebuild app/render light state from durable building
  data. This is an explicit Rust cache rebuild, not serialization of a gamemd
  runtime handle.

## Doc-Patch-Ready Facts

- Replace prior wording that `+0x614` is "recreated on-demand during the first
  post-load tick" with: load zeroes `+0x614`; no general lazy rehydrate caller
  was verified; Rust should rebuild runtime light state explicitly after load.
- Replace claims that `+0x614` is default/all-building ambient light with:
  building `LightSourceClass` allocation is gated by nonzero `LightIntensity`.
- Replace `ExtraLight=` ambience/brightness wording with signed building
  draw-depth/Z adjustment.
- Replace Rust-facing comments that say the current point-light calculation
  matches original behavior; it is only a provisional approximation.

## Stale Or Superseded Claims

- `MAP_LIGHTING_FINAL_SYSTEM_MODEL_SYNTHESIS.md` still lists exact post-load
  `+0x614` rehydrate caller as needing `/re-investigate`. Superseded by
  `BUILDING_LIGHTSOURCE_POST_LOAD_REHYDRATE_GHIDRA_REPORT.md`: the static slice
  found no general caller and turns this into a correction bundle plus optional
  runtime measurement target.
- `BUILDINGCLASS_SAVE_LOAD_GHIDRA_REPORT.md` and
  `BUILDINGCLASS_UNLIMBO_AND_PLACEMENT.md` wording about first-tick lazy rebuild
  is stale for `+0x614`.
- Current Rust in `src/map/lighting.rs` remains stale against the verified model:
  wrong ground formula/default, approximate point light math, and `ExtraLight`
  applied as RGB brightness.

## Cross-Doc Conflicts

- "Ambient" must be disambiguated:
  - scenario ambience: map `[Lighting]`;
  - radius ambience: `LightSourceClass+0x614`;
  - spotlight/searchlight: `BuildingLightClass+0x600`;
  - art draw-depth delta: `ExtraLight=`.
- Missing tint keys preserve raw constructor `1000000`, while explicit `1.0`
  stores `1000`. Downstream equivalence is likely but not safe to erase until
  the LightSource/LightConvert scale contract is represented in Rust.

## Needs Re-Investigation

- `/re-investigate BuildingLightClass beam rasterization and ProcessCellAction 0x23`
  only when implementing `HasSpotlight=` beams.
- `/re-investigate ConvertClass low-level palette table generation for LightConvert`
  only when byte-exact palette-table generation is required.
- Optional runtime trace: stock save/load with a visible lamp building, to measure
  whether the final visible light is restored even though no general static
  rehydrate caller was found.

## Do-Not-Implement Notes

- Do not implement this through Lightning Storm/weather state.
- Do not use `ExtraLight / 1000.0` as RGB brightness.
- Do not create light sources from `LightVisibility` alone.
- Do not parse verified lamp `0,01` as `0.01`.
- Do not serialize `LightSourceClass` handles.
- Do not collapse spotlight beams and radius lamps into one lighting system.
- Do not put render/palette types into `sim/`; dynamic light invalidation must
  cross the sim/render boundary through data/events, not dependencies.

## Source Ledger

- Reconciliations: `MAP_LIGHTING_RE_SWARM_RECONCILIATION_2026_05_22.md`,
  `MAP_LIGHTING_RE_SWARM_ROUND2_RECONCILIATION_2026_05_22.md`,
  `.swarm-claims.md`.
- Latest targeted report: `BUILDING_LIGHTSOURCE_POST_LOAD_REHYDRATE_GHIDRA_REPORT.md`.
- Implementation handoffs: `MAP_LIGHTING_IMPLEMENTATION_SPEC.md`,
  `MAP_LIGHTING_RUST_HANDOFF_AUDIT.md`.
- Core Ghidra reports: `SCENARIO_LIGHTING_FIELDS_00689E90_GHIDRA_REPORT.md`,
  `SCENARIO_LIGHTING_DEFAULT_RESET_PATH_GHIDRA_REPORT.md`,
  `MAP_LIGHTING_CELL_COMPUTE_00484180_GHIDRA_REPORT.md`,
  `MAP_LIGHTCONVERT_CACHE_00483E30_00544E70_GHIDRA_REPORT.md`,
  `LIGHTCONVERT_NORMALIZE_005558E0_00555AC0_GHIDRA_REPORT.md`,
  `BUILDINGTYPE_LIGHT_KEYS_READINI_CONSTANTS_GHIDRA_REPORT.md`,
  `LIGHTSOURCE_QUEUED_MODE_CALLER_CENSUS_GHIDRA_REPORT.md`,
  `LIGHTSOURCE_LIFECYCLE_POWER_DAMAGE_SAVELOAD_GHIDRA_REPORT.md`,
  `BUILDINGTYPE_EXTRALIGHT_0X1548_CONSUMER_GHIDRA_REPORT.md`,
  `EXTRALIGHT_DRAWBODY_Z_DELTA_DETAILS_GHIDRA_REPORT.md`,
  `LIGHTING_DRAW_CONSUMERS_CELL_FIELDS_GHIDRA_REPORT.md`,
  `BUILDINGLIGHTCLASS_SPOTLIGHT_PATH_GHIDRA_REPORT.md`.
- Spot-checked Ghidra addresses: `0x00554760`, `0x00454174`, `0x00446759`,
  `0x00449AD0..0x00449AD4`.
- Current Rust surfaces: `src/map/lighting.rs`, `src/app_init.rs`,
  `src/rules/object_type.rs`, `src/rules/art_data.rs`,
  `src/rules/ini_parser.rs`, `src/app_input.rs`, `src/sim/world/mod.rs`.

## Classification

Implementation-safe for ordinary map ambience, static lamp/radius lights,
parser/default fixes, removal of `ExtraLight=` from RGB lighting, LightConvert
profile shape, and explicit Rust post-load render-light cache rebuild.

Doc-patch-ready for stale `+0x614` first-post-load rehydrate wording.

Investigation-blocked only for spotlight beam rasterization, byte-exact
ConvertClass palette internals, and optional runtime measurement of stock
save/load lamp visibility.
