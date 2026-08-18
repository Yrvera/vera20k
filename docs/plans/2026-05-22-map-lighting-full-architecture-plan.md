# Map Lighting Full Architecture Implementation Plan

> **For Claude:** Execute this plan task-by-task. Each task is self-contained and includes the files to inspect, the intended change, and the verification to run before moving on. Do not broaden this into Lightning Storm, spotlights, fog of war, or palette-table reverse engineering.

## Goal

Implement ordinary Red Alert 2 / Yuri's Revenge map lighting parity for ambient map light and building light-post style point lights, including:

- the immediate correctness fixes for current RGB lighting behavior;
- a `CellLight` / `LightConvert`-style profile cache model;
- post-load light-state rebuild;
- render consumer migration away from raw `HashMap<(u16, u16), [f32; 3]>` tint reads;
- removal of the incorrect `ExtraLight=` RGB brightening path.

This plan is for implementation only. It relies on the already completed research and synthesis documents.

## Design Reference

- Design doc: `docs/plans/2026-05-22-map-lighting-full-architecture-design.md`
- Final synthesis: `docs/research/MAP_LIGHTING_POST_REINVESTIGATION_SYSTEM_MODEL_SYNTHESIS.md`
- Reinvestigation: `docs/research/BUILDING_LIGHTSOURCE_POST_LOAD_REHYDRATE_GHIDRA_REPORT.md`
- Initial system model: `docs/research/MAP_LIGHTING_AND_LIGHT_POSTS_SYSTEM_MODEL_SYNTHESIS.md`

## Grounding Summary

The current Rust implementation has a useful render-facing tint grid, but it does not match the binary model:

- `[Lighting]` `Ground` defaults to `0.0`; YR default is `0.20`.
- Current ambient math multiplies by `(1.0 - Ground)`; YR uses additive/scalar math shaped like `Ambient + Level * cell_level - Ground`.
- `ExtraLight=` is applied as an RGB brightness boost; YR uses it as a signed body depth/Z scalar for building drawing, not color.
- Building light creation is effectively gated by nonzero `LightIntensity`, while `LightVisibility` defaults to `5000` leptons.
- `LightGreenTint=0,01` in stock data must parse/store as `0`, not fail into a Rust-specific fallback that hides the distinction.
- Point lights are lepton-radius, cell-center based, signed, summed before normalization/clamp, and support negative contribution.
- Save/load must rebuild transient light-source/render state. The binary zeroes a transient building light handle during load and rehydrates it after object load.
- Render code reads raw RGB tint from `AppState.lighting_grid`; this needs a compatibility step before consumer-specific migration.

## Architecture Target

Keep lighting outside `sim/`. Simulation owns deterministic entity state; map/app/render code derives transient light/render state from map, rules, and sim entities.

Target shape:

- `src/map/lighting.rs`
  - owns `LightingConfig`, `PointLight`, `CellLight`, `CellLightGrid`, `LightProfileCache`, and render-facing light accessors;
  - exposes compatibility tint methods while render code migrates;
  - does not depend on render, ui, audio, net, or sim.
- `src/app_init.rs`
  - constructs app lighting on map load through one helper.
- `src/app_input.rs`
  - calls the same helper after save load / entity reconstruction.
- `src/app_instances/*.rs`
  - reads lighting through named accessors instead of direct map lookup.
- `src/rules/object_type.rs`
  - parses light keys with YR-compatible defaults and numeric behavior.
- `src/rules/art_data.rs`
  - documents and exposes `ExtraLight=` as draw-depth input, not RGB light.

## Key Technical Decisions

- Preserve the current `SpriteInstance.tint: [f32; 3]` shader ABI for the first implementation. The profile cache feeds existing tint uniforms through accessors.
- Add `CellLightGrid` before changing shader or palette internals.
- Use integer-ish binary units internally for verified lighting math:
  - RGB and ambient normalized around `1000 == 1.0`;
  - INI `Ambient/Red/Green/Blue` scale by `100`;
  - INI `Ground/Level` scale by `250`;
  - light intensity/tints scale by `1000`.
- Store point-light radius in leptons and calculate distance from cell centers.
- Clamp once after total contribution is accumulated, not per light source.
- Treat `LightConvert` parity as a profile identity/cache model now, not byte-exact palette remapping.
- Defer spotlights and low-level beam rasterization. They are not required for ordinary lamp ambience.

## File Map

Primary files:

- `src/map/lighting.rs`
- `src/app.rs`
- `src/app_init.rs`
- `src/app_input.rs`
- `src/app_instances/shp.rs`
- `src/app_instances/units.rs`
- `src/app_instances/overlays.rs`
- `src/app_instances/bridges.rs`
- `src/rules/object_type.rs`
- `src/rules/ini_parser.rs`
- `src/rules/art_data.rs`

Render ABI files to inspect but avoid changing unless the migration proves necessary:

- `src/render/batch.rs`
- `src/render/batch_shader.wgsl`
- `src/render/sprite_voxel_shader.wgsl`

Useful validation data:

- `ini/rules.ini`
- `ini/rulesmd.ini`
- `ini/art.ini`
- `ini/artmd.ini`

## Parity-Critical Items

Do not lose these details during implementation:

- `Ambient`, `Red`, `Green`, `Blue` default to `1.00`.
- `Ground` defaults to `0.20`.
- `Level` defaults to `0.032`.
- Ambient math is additive, not multiplicative.
- `LightVisibility` default is `5000` leptons.
- Building light existence is gated by nonzero `LightIntensity`.
- `LightIntensity`, `LightRedTint`, `LightGreenTint`, `LightBlueTint` store `ftol(value * 1000 + 0.1)`.
- Stock comma decimal `0,01` should parse as `0` for these light tint fields.
- Radius test is inclusive.
- Contributions may be negative.
- Division truncates toward zero.
- Sum contributions before normalizing/clamping.
- `ExtraLight=` never mutates RGB tint.
- Post-load light state is transient and must be rebuilt from loaded entities.

## Task 1 - Baseline And Guardrails

Inspect the current call sites and tests before editing:

- `src/map/lighting.rs`
- `src/app_init.rs`
- `src/app_input.rs`
- `src/app_instances/shp.rs`
- `src/app_instances/units.rs`
- `src/app_instances/overlays.rs`
- `src/app_instances/bridges.rs`
- `src/rules/object_type.rs`
- `src/rules/ini_parser.rs`
- `src/rules/art_data.rs`

Actions:

- Record which tests currently cover lighting, object light parsing, and render instance tint behavior.
- Confirm whether any current tests intentionally encode the wrong `ExtraLight=` RGB behavior.
- Check current git status and do not touch unrelated dirty files.

Verification:

- Run the narrow existing lighting/rules tests once, even if they are expected to change.
- Run `cargo check` only if the workspace is already in a known-good state; otherwise record the pre-existing failure.

## Task 2 - Remove Incorrect ExtraLight RGB Lighting

Files:

- `src/map/lighting.rs`
- `src/app_init.rs`
- `src/rules/art_data.rs`
- any tests in `src/map/lighting.rs` or nearby modules that assert RGB brightening from `ExtraLight=`.

Actions:

- Stop calling `apply_extra_light` from app map-light construction.
- Remove, retire, or narrow `apply_extra_light` so it cannot be used as an RGB lighting path.
- Update `art_data.rs` comments for `ExtraLight=` to describe it as signed building body draw-depth/Z input, not ambient brightness.
- Replace tests that assert RGB brightening with tests that assert `ExtraLight=` does not affect cell RGB tint.

Acceptance:

- A map object with `ExtraLight=` no longer changes the RGB tint returned for that cell.
- Existing unit/infantry/aircraft extra-light rules remain untouched; those are separate render brightness adjustments.

Verification:

- Run focused lighting tests.
- Run focused art-data parsing tests if they exist.

## Task 3 - Fix LightingConfig Defaults And Base Formula

Files:

- `src/map/lighting.rs`
- tests in the same module or test module.

Actions:

- Change `LightingConfig::default()` to match YR defaults:
  - `ambient = 1.0`
  - `red = 1.0`
  - `green = 1.0`
  - `blue = 1.0`
  - `ground = 0.20`
  - `level = 0.032`
- Replace multiplicative ground darkening with the additive model:
  - conceptual formula: `ambient + level * cell_level - ground`
  - apply the same scalar to red/green/blue channels before later point-light contribution.
- Keep the public compatibility output as `[f32; 3]` for now.
- Add named constants for binary unit scales so later tasks can reuse them.

Acceptance:

- Default ground-level cells produce the YR-style darkened neutral tint, not full-white default and not multiplicative dimming.
- Higher terrain levels change by `Level * z`.
- Channel tinting still honors Red/Green/Blue scalars.

Verification:

- Add or update unit tests for defaults, level contribution, ground subtraction, and channel multipliers.

## Task 4 - Add YR-Compatible Light Numeric Parsing

Files:

- `src/rules/ini_parser.rs`
- `src/rules/object_type.rs`
- related parser tests.

Actions:

- Add a scoped helper for Westwood-style light float parsing rather than changing every `get_f32` caller globally.
- The helper must preserve the verified behavior needed for stock `LightGreenTint=0,01`: parse/store as `0`.
- Update object type parsing for:
  - `LightIntensity`
  - `LightRedTint`
  - `LightGreenTint`
  - `LightBlueTint`
- Change `LightVisibility` default from `0` to `5000`.
- Keep `LightIntensity` default at `0.0`.
- Update field comments around `LightVisibility` and light tints.

Acceptance:

- A section with no `LightVisibility` yields `5000`.
- A section with no `LightIntensity` still produces no light.
- `LightGreenTint=0,01` stores as `0.0` or equivalent zero light-tint value in the parsed object type.
- Valid decimal values with `.` still parse normally.

Verification:

- Add focused parser/object-type unit tests.
- Run object-type parser tests and lighting tests.

## Task 5 - Introduce CellLightGrid And LightProfileCache Types

Files:

- `src/map/lighting.rs`

Actions:

- Add `LightProfileId`, `LightProfile`, and `LightProfileCache`.
- Key profiles by normalized RGB triple, not by cell coordinate or source object.
- Ensure the neutral/default profile is stable and reused.
- Add `CellLight` with at least:
  - profile id;
  - top scalar;
  - common scalar;
  - bottom scalar;
  - RGB key/triple used for profile lookup.
- Add `CellLightGrid` as the new map-level lighting container.
- Keep the old direct tint behavior available through compatibility methods:
  - `tint_at(cell) -> [f32; 3]`
  - `tint_or_default(cell) -> [f32; 3]`
  - or equivalent names matching repo style.

Acceptance:

- The new types can be unit-tested without changing app/render call sites yet.
- Profile cache deduplicates identical RGB triples.
- Default/no-entry cells resolve to the neutral/default profile.

Verification:

- Unit tests for profile reuse, default profile, and compatibility tint conversion.

## Task 6 - Build Base CellLightGrid From Map Lighting

Files:

- `src/map/lighting.rs`
- `src/app_init.rs` only if needed for a temporary builder call.

Actions:

- Add a builder that constructs a `CellLightGrid` from:
  - map dimensions or known cell set;
  - terrain heights;
  - `LightingConfig`.
- Populate base top/common/bottom scalars and RGB profile keys using the fixed `[Lighting]` math.
- Keep output compatible with existing render tint while the app still stores the old grid type.

Acceptance:

- Base grid construction has the same visible tint output as Task 3's fixed compatibility path.
- The profile cache contains one profile for neutral/default lighting unless channel values require more.

Verification:

- Unit tests compare old compatibility calculation and new `CellLightGrid` tint output for flat and elevated cells.

## Task 7 - Switch AppState To CellLightGrid Compatibility Access

Files:

- `src/app.rs`
- `src/app_init.rs`
- `src/app_instances/shp.rs`
- `src/app_instances/units.rs`
- `src/app_instances/overlays.rs`
- `src/app_instances/bridges.rs`

Actions:

- Change `AppState.lighting_grid` from the raw `LightingGrid` alias to `CellLightGrid`.
- Replace direct `.get(&(x, y)).copied().unwrap_or(DEFAULT_TINT)` call sites with compatibility methods.
- Preserve all existing non-map-light render adjustments:
  - `ExtraUnitLight`
  - `ExtraInfantryLight`
  - `ExtraAircraftLight`
  - current neutral-tint exceptions such as shadows/parachutes where intentional.
- Keep shader inputs unchanged.

Acceptance:

- Render instance builders compile with no raw `HashMap` lighting access.
- Visual output should match the Task 3/6 compatibility tint behavior before point-light math changes.

Verification:

- `cargo check`
- focused render instance tests if present.

## Task 8 - Implement Binary-Shaped PointLight Model

Files:

- `src/map/lighting.rs`
- `src/rules/object_type.rs` if additional conversion helpers are needed.

Actions:

- Change `PointLight` representation to use:
  - radius in leptons;
  - signed intensity in `1000 == 1.0` units;
  - signed RGB tint components in `1000 == 1.0` units;
  - active/detail flags if practical within the current Rust architecture.
- Convert parsed object type light fields into the verified integer-ish units with `value * 1000 + 0.1` semantics.
- Use building/map object cell center as `cell * 256 + 128`.
- Keep the collection gate as nonzero `LightIntensity`.
- Do not reject a light only because `LightVisibility` is omitted; default is already `5000`.

Acceptance:

- Buildings with nonzero intensity and omitted visibility create a light with `5000` lepton radius.
- Buildings with visibility but zero intensity create no light.
- Negative intensity/tints remain representable.

Verification:

- Unit tests for collection gate, default radius, explicit radius, and signed values.

## Task 9 - Replace Point-Light Accumulation Math

Files:

- `src/map/lighting.rs`
- tests in the same module.

Actions:

- Replace f32 cell-distance falloff with lepton-center distance.
- Use inclusive radius comparison.
- Accumulate signed contributions per channel before normalization/clamp.
- Apply truncation toward zero where division is required.
- Clamp only after all source contributions and base light have been summed.
- Route contributions into `CellLightGrid` profiles so cells with identical RGB triples reuse profiles.
- Respect active/detail gate fields if they are represented.

Acceptance:

- A cell exactly on the radius boundary receives the verified edge behavior.
- Multiple weak lights sum before clamp.
- Negative lights can darken a cell.
- Per-source clamp is gone.

Verification:

- Unit tests for boundary inclusion, signed darkening, sum-before-clamp, and truncation behavior.
- Run full `src/map/lighting.rs` test set.

## Task 10 - Factor Shared App Lighting Rebuild Helper

Files:

- `src/app_init.rs`
- `src/app_input.rs`
- possibly a small helper module if `app_init.rs` would become too large.

Actions:

- Extract map/app lighting construction into one reusable helper.
- Inputs should include the map/rules/terrain data and current entity set required to collect live building lights.
- Keep the helper app-side; do not move it into `sim/`.
- Update initial map load to call the helper.
- Ensure the helper can be called after save load, when transient render caches are rebuilt.

Acceptance:

- Initial map loading uses the same construction path as post-load rebuild.
- No sim module imports render/app lighting types.
- No duplicated point-light collection path remains in `app_init.rs` and `app_input.rs`.

Verification:

- `cargo check`
- focused app init/load tests if available.

## Task 11 - Rehydrate Lighting After Save Load

Files:

- `src/app_input.rs`
- helper from Task 10.

Actions:

- After `GameSnapshot` load and entity cache rebuild, call the shared lighting rebuild helper.
- Collect building lights from the loaded sim/entity state, not only from original map object data.
- Ensure destroyed/missing buildings do not contribute lights after load.
- Ensure buildings present in the loaded state with nonzero light intensity do contribute.
- Rebuild profile cache as transient state; do not serialize profile ids or transient handles.

Acceptance:

- Save/load does not leave stale map-start lights for destroyed buildings.
- Save/load restores lights for surviving light-source buildings.
- There is no serialized dependency on `LightSource+0x614`-style transient handles.

Verification:

- Add a focused test if the save/load harness allows it:
  - create/load state with a light-source building;
  - destroy/remove it;
  - rebuild;
  - assert light contribution changes.
- At minimum, add unit-level helper tests for collecting from current entities.

## Task 12 - Add Consumer-Specific Lighting Accessors

Files:

- `src/map/lighting.rs`
- maybe a small sibling module if this file grows too large.

Actions:

- Add named accessors for render consumers:
  - `techno_tint_at`
  - `unit_tint_at` or shared techno + caller-added extra glow
  - `infantry_tint_at` or shared techno + caller-added extra glow
  - `aircraft_tint_at` or shared techno + caller-added extra glow
  - `overlay_tint_at`
  - `terrain_object_tint_at`
  - `anim_tint_at`
  - `bridge_body_tint_at`
  - `building_body_depth_adjustment`
- Keep accessors simple and backed by the same `CellLight` values until a verified consumer difference requires a split.
- Return depth adjustment as a signed scalar, not RGB.

Acceptance:

- Accessor names document the intended render consumer without changing shader ABI.
- `building_body_depth_adjustment` is testable independently from RGB tint.

Verification:

- Unit tests for accessors returning expected compatibility values and `ExtraLight=` depth scalar.

## Task 13 - Migrate SHP And Techno Render Consumers

Files:

- `src/app_instances/shp.rs`
- `src/app_instances/units.rs`

Actions:

- Replace generic compatibility tint reads with consumer-specific accessors.
- Preserve existing extra render glow additions:
  - infantry: `rules.general.extra_infantry_light`;
  - units: `rules.general.extra_unit_light`;
  - aircraft: `rules.general.extra_aircraft_light`.
- For buildings:
  - use building/body tint accessor for RGB;
  - thread `building_body_depth_adjustment` into draw ordering only if the current renderer has a clear, exact mapping point;
  - if not, keep the helper wired and documented, and do not approximate depth behavior.

Acceptance:

- Units/infantry/aircraft still receive their non-map-light extra brightness.
- Buildings no longer rely on `ExtraLight=` for RGB.
- Any draw-depth change is explicit and covered by a test or screenshot scenario.

Verification:

- `cargo check`
- focused render instance tests if present.

## Task 14 - Migrate Overlay, Terrain Object, Bridge, And Effect Consumers

Files:

- `src/app_instances/overlays.rs`
- `src/app_instances/bridges.rs`

Actions:

- Replace remaining generic compatibility tint reads with named accessors.
- Keep known neutral paths neutral:
  - bridge shadows;
  - parachutes or other intentionally full-bright helper sprites if current behavior is intentional.
- Use terrain-object accessor for tree/terrain object SHPs.
- Use overlay/effect accessor for overlays, damage fires, muzzle flashes, projectiles, and similar sprites unless research says they should be full-bright.

Acceptance:

- No render consumer directly indexes the lighting grid.
- Neutral exceptions are visible in code as intentional decisions, not accidental missing lighting.

Verification:

- `cargo check`
- run any overlay/bridge render tests if present.

## Task 15 - Cleanup Old Lighting API And Comments

Files:

- `src/map/lighting.rs`
- `src/app_instances/*.rs`
- `src/rules/art_data.rs`
- `src/rules/object_type.rs`
- relevant docs/comments.

Actions:

- Remove unused old `LightingGrid = HashMap<(u16, u16), [f32; 3]>` alias if no longer needed.
- Remove stale comments describing the old approximation as original behavior.
- Ensure module doc comments describe ownership and boundaries.
- Keep file size in mind; split `lighting.rs` only if it has clearly outgrown a cohesive module.

Acceptance:

- No stale comments claim `ExtraLight=` is RGB brightness.
- No call site performs raw lighting map lookup.
- Public names match the architecture.

Verification:

- `rg "ExtraLight" src`
- `rg "lighting_grid\.get|HashMap<\(u16, u16\), \[f32; 3\]" src`
- `cargo check`

## Task 16 - Final Validation

Run the strongest affordable verification set.

Required:

- `cargo fmt`
- `cargo check`
- focused lighting tests
- focused rules/object type parser tests
- any save/load tests touched by Task 11

Recommended if runtime is reasonable:

- `cargo test map::lighting`
- `cargo test rules`
- a broader `cargo test` if the workspace is in a healthy state.

Manual/visual scenarios:

- Flat map with no lamps: default lighting should be subtly ground-darkened, not full-white.
- Elevated terrain: level contribution should be visible in the expected direction.
- Map with light-post buildings: cells inside radius should tint, boundary should be inclusive.
- Negative or tinted light test map if available: signed contribution should behave visibly.
- Save/load after destroying a light-source building: destroyed light must not remain.
- `ExtraLight=` building: no RGB brightening should occur from that key.

## Deferred Work

Do not implement these in this pass:

- Lightning Storm superweapon lighting.
- Spotlight beam rasterization.
- Byte-exact `ConvertClass` palette-table internals.
- Shader ABI redesign for palette-indexed lighting.
- TS fog-of-war behavior unless a YR-visible scenario requires it.

## Suggested Commit Boundaries

Use these boundaries if committing later:

1. Parser/default/formula fixes and `ExtraLight=` RGB removal.
2. `CellLightGrid` and profile cache introduction.
3. Point-light binary math and app-state migration.
4. Save/load lighting rebuild.
5. Render consumer accessor migration and cleanup.

