# Map Lighting Full Architecture Design

## Goal

Implement the verified Yuri's Revenge ordinary map lighting and lamp/light-post ambience model, including a `CellLight`/LightConvert-style profile layer and render consumer migration, without mixing in Lightning Storm, spotlights, or byte-exact ConvertClass palette internals.

## Architecture Context

Current Rust lighting is app/render-side state. `src/app_init.rs` builds `AppState.lighting_grid` once at map load from `src/map/lighting.rs`. Render builders in `src/app_instances/shp.rs`, `src/app_instances/units.rs`, `src/app_instances/overlays.rs`, and `src/app_instances/bridges.rs` look up a cell `[f32; 3]` tint and pass it through `SpriteInstance.tint` into the sprite/voxel WGSL shaders.

Rules parsing lives in `src/rules/object_type.rs` and `src/rules/ini_parser.rs`. `ExtraLight=` parsing lives in `src/rules/art_data.rs`. Save/load restoration in `src/app_input.rs::load_save_file` rebuilds sim caches and atlases, but does not rebuild app/render lighting.

The architecture boundary is important: `sim/` must not depend on render, palette, or app lighting. Full lighting should live in `map/` data/model code plus app/render orchestration. Later dynamic lighting can use sim-emitted state/events, but the render cache remains outside `sim/`.

## Impact Analysis

Touched files/modules:

- `src/map/lighting.rs`: replace the current tint-only model with binary-shaped lighting math and a `CellLight`/profile cache.
- `src/app_init.rs`: build the new lighting grid at map load and stop applying `ExtraLight` as RGB.
- `src/app_input.rs`: rebuild app/render lighting after snapshot load.
- `src/rules/object_type.rs`: fix light defaults and use scoped Westwood float parsing for verified light keys.
- `src/rules/ini_parser.rs`: add a parser helper for Westwood/`sscanf("%f")`-style prefix floats.
- `src/rules/art_data.rs`: keep parsing `ExtraLight`, but document it as signed building body depth/Z adjustment.
- `src/app_instances/*.rs`: migrate consumers from raw tint lookup to consumer-specific `CellLight` accessors.
- `src/render/batch.rs` and WGSL shaders: only changed if/when profile/scalar fields need GPU-side representation.

Risk areas:

- `SpriteInstance.tint` is a broad render contract. A big-bang ABI change would affect SHP, VXL, overlays, bridges, projectiles, effects, and terrain-like objects at once.
- Current tests encode some wrong behavior, especially `ExtraLight` as RGB brightness and multiplicative ground darkening.
- The scoped float parser must not be applied globally unless each key family is verified.
- Post-load lighting rebuild must stay app/render-side; it should not make `sim::world::rebuild_caches_after_load` own render data.

## Chosen Approach

Use a staged full architecture with a compatibility adapter.

1. First remove known wrong behavior and fix parser/defaults/math while preserving the current `[f32; 3]` render contract.
2. Introduce `CellLightGrid` and `LightProfileCache` in `src/map/lighting.rs`.
3. Expose compatibility tint accessors so current render paths can keep working while the new model becomes authoritative.
4. Migrate render consumers by category to explicit accessors such as `techno_tint`, `overlay_tint`, `terrain_object_tint`, and `building_body_depth_adjustment`.
5. Only change the `SpriteInstance`/WGSL ABI if a later phase needs profile/scalar fields on the GPU. Until byte-exact palette tables are implemented, CPU-side profile-to-tint adaptation is acceptable.

This avoids a renderer big bang while still building toward the verified architecture.

## Tiny-Detail Ledger

- Missing `[Lighting]` defaults are `Ambient/R/G/B=1.00`, `Ground=0.20`, `Level=0.032`. Source: `SCENARIO_LIGHTING_DEFAULT_RESET_PATH_GHIDRA_REPORT.md`.
- Ordinary brightness is additive: `Ambient + Level * cell_level - Ground`, not `Ambient * (1 - Ground)`. Source: `SCENARIO_LIGHTING_FIELDS_00689E90_GHIDRA_REPORT.md`.
- Binary scales author values before compute: `Ambient/Red/Green/Blue * 100`, `Ground/Level * 250`, with cell compute in `1000 == 1.0` units. Source: `MAP_LIGHTING_IMPLEMENTATION_SPEC.md` source ledger.
- Building light allocation/collection is gated by nonzero `LightIntensity`, not `LightVisibility`. Source: `BUILDINGTYPE_LIGHT_KEYS_READINI_CONSTANTS_GHIDRA_REPORT.md`; Ghidra caller spot-check for `LightSourceClass__Constructor @ 0x00554760`.
- `LightVisibility` default is `5000` leptons. Source: `BUILDINGTYPE_LIGHT_KEYS_READINI_CONSTANTS_GHIDRA_REPORT.md`; `ini/rules.ini` light-key comments.
- `LightIntensity` and tint values store `ftol(value * 1000 + 0.1)`. Source: `BUILDINGTYPE_LIGHT_KEYS_READINI_CONSTANTS_GHIDRA_REPORT.md`.
- Stock `LightGreenTint=0,01` parses/stores as `0`, not `0.01`. Source: `BUILDINGTYPE_LIGHT_KEYS_READINI_CONSTANTS_GHIDRA_REPORT.md`; stock `ini/rules.ini`.
- Point lights use cell centers `cell * 256 + 128`, radius in leptons, and inclusive radius checks. Source: `MAP_LIGHTING_CELL_COMPUTE_00484180_GHIDRA_REPORT.md`.
- Point-light contribution is signed; negative lamps are valid. Division truncates toward zero. Source: `MAP_LIGHTING_CELL_COMPUTE_00484180_GHIDRA_REPORT.md`.
- Light contributions sum before normalization/clamp; do not clamp per source. Source: `MAP_LIGHTING_CELL_COMPUTE_00484180_GHIDRA_REPORT.md`.
- `LightSource+0x48` active state and `LightSource+0x34` detail threshold gate contribution. Source: `MAP_LIGHTING_CELL_COMPUTE_00484180_GHIDRA_REPORT.md`.
- LightConvert cache key is the normalized RGB triple, not cell coordinate, height, scalar brightness, or source identity. Source: `MAP_LIGHTCONVERT_CACHE_00483E30_00544E70_GHIDRA_REPORT.md`.
- Cell lighting has separate scalar consumers: `Cell+0x10A`, `Cell+0x10C`, `Cell+0x10E`, and RGB key mirror `Cell+0x110/+0x112/+0x114`. Source: `LIGHTING_DRAW_CONSUMERS_CELL_FIELDS_GHIDRA_REPORT.md`.
- `ExtraLight=` is signed `Cell+0x10A + BuildingType+0x1548` building body depth/Z adjustment, not RGB brightness. Source: `EXTRALIGHT_DRAWBODY_Z_DELTA_DETAILS_GHIDRA_REPORT.md`.
- `BuildingClass::Load` zeroes `+0x614`; Rust must rebuild render light state after load rather than serialize a runtime light handle. Source: `BUILDING_LIGHTSOURCE_POST_LOAD_REHYDRATE_GHIDRA_REPORT.md`; Ghidra `0x00454174`.

## Design

### Components

#### `LightingConfig`

Keep `LightingConfig` as the parsed map `[Lighting]` ordinary-lighting config, but correct its defaults and comments. Add binary-unit helpers so tests can verify source unit scaling separately from compatibility float output.

Required behavior:

- `Ground` default is `0.20`.
- `cell_tint` uses additive brightness.
- Ion/Nuke/Dominator/Lightning transition fields remain non-scope and must not be folded into ordinary lighting.

#### `PointLight`

Change `PointLight` from float cell-space range to binary-shaped source data:

- `rx`, `ry`
- `radius_leptons`
- signed intensity in `1000 == 1.0` units, or a wrapper that preserves equivalent integer math
- signed tint fields in the same scale contract used by the report
- active/detail fields with default active true and threshold `2` for static map-load lights

Collection remains app/map-side and uses rule/map entities. The creation gate is `LightIntensity != 0`.

#### `CellLight`

Introduce a binary-shaped cell-light bundle:

```text
CellLight {
  profile_id,
  top,
  common,
  bottom,
  rgb_key,
}
```

Names can differ, but the fields must represent:

- `top`: `Cell+0x10A`-like scalar
- `common`: `Cell+0x10C`-like scalar
- `bottom`: `Cell+0x10E`-like scalar
- `rgb_key`: normalized RGB mirror from `Cell+0x110/+0x112/+0x114`
- `profile_id`: LightConvert-style cache entry keyed by normalized RGB triple

Keep `LightingGrid` temporarily as either:

- a compatibility type alias to the new grid plus tint accessor, or
- `CellLightGrid` plus explicit call-site migration.

The preferred implementation is to add `CellLightGrid` and keep compatibility methods like `common_tint_at(rx, ry)`.

#### `LightProfileCache`

Add a deterministic cache keyed by normalized/quantized RGB triple.

Requirements:

- Default full-bright profile for `(1000,1000,1000)`.
- Key ignores cell coordinate, height, scalar brightness, and source identity.
- Normalization clamps channels to `0..1000`.
- Detail-level quantization belongs here or in a helper called before insertion.

This is not a byte-exact ConvertClass palette table yet. It is the render-facing profile shape the current code can route through.

#### Render Accessors

Add accessor methods that make consumer intent explicit:

- `techno_tint_at(rx, ry)` -> uses common scalar/profile.
- `overlay_tint_at(rx, ry, branch)` -> starts with common, gains top/bottom branch support as overlay branches are migrated.
- `terrain_object_tint_at(rx, ry, branch)` -> starts with common, later supports verified top branch.
- `anim_tint_at(rx, ry, anim_light_mode)` -> avoids applying cell lighting to anims known to use fixed/global paths.
- `building_body_depth_adjustment(rx, ry, art_extra_light)` -> signed scalar for draw-depth/Z use, not RGB tint.

During migration these can return the current compatibility tint while tests lock the intended routing.

### Interfaces / Contracts

`src/map/lighting.rs` should expose:

- `parse_lighting(&IniFile) -> LightingConfig`
- `build_cell_light_grid(...) -> CellLightGrid`
- `collect_building_lights(...) -> Vec<PointLight>`
- `accumulate_point_lights(&mut CellLightGrid, &[PointLight], detail_level)`
- compatibility accessors returning `[f32; 3]`

`src/app_init.rs` should own construction order:

1. Parse ordinary map lighting.
2. Build base `CellLightGrid`.
3. Collect and accumulate static building lights.
4. Store `CellLightGrid`/profile cache in `AppState`.
5. Do not apply `ExtraLight` to RGB.

`src/app_input.rs::load_save_file` should rebuild lighting after loaded sim replacement and cache restoration, using the same app-side helper as map init where possible. The helper should derive lights from loaded simulation entities rather than serialized light handles.

`src/rules/ini_parser.rs` should expose a narrowly named helper such as `get_westwood_f32_prefix` or `get_light_f32`. `object_type.rs` should use it only for verified light fields at first.

### Data Flow

Initial map load:

```text
map INI + resolved terrain
  -> LightingConfig
  -> base CellLightGrid
rules + map entities
  -> PointLight sources
  -> accumulate into CellLightGrid
CellLightGrid + LightProfileCache
  -> AppState
render builders
  -> consumer-specific accessors
  -> SpriteInstance tint/depth fields
```

Snapshot load:

```text
GameSnapshot::load
  -> sim.rebuild_caches_after_load(...)
  -> rebuild_dynamic_path_grid(...)
  -> refresh_entity_atlases(...)
  -> rebuild app/render lighting from loaded sim entities + rules + preserved map lighting config/terrain
```

The app must retain enough base map lighting inputs after map load to rebuild lighting later: either `LightingConfig` plus resolved terrain/cell levels, or a reusable base grid before dynamic lights.

### Error Handling

- Missing rules: build only base scenario lighting and log once.
- Missing art registry: no `ExtraLight` depth adjustment until building-body integration; do not touch RGB.
- Missing lighting config on save/load rebuild: log and preserve existing lighting grid rather than panic.
- Unknown out-of-grid light cells: skip cells outside the grid; do not synthesize cells.
- Parser helper failure: fall back to verified default for that key, not to Rust strict parse behavior.

### Testing Strategy

Unit tests in `src/map/lighting.rs`:

- missing-key defaults
- additive formula
- scale conversions
- point light center/radius/inclusive edge
- negative light truncation toward zero
- sum-before-clamp
- detail gate
- profile key ignores coordinate and height
- profile key clamps/quantizes RGB
- compatibility tint returns expected common scalar/profile output

Rules/parser tests:

- `LightVisibility` defaults to `5000`
- `LightIntensity == 0` prevents light collection despite default visibility
- `0,01` parses as `0.0` for light keys
- strict `get_f32` behavior remains unchanged for unverified keys if helper is scoped

App/render tests where practical:

- `ExtraLight` does not mutate RGB grid
- building-body depth adjustment helper adds signed `ExtraLight` to top scalar
- save/load rebuild regenerates point light state without serialized light handles

Screenshot/visual verification:

- map with stock lamp posts before/after point-light math
- map with buildings using `ExtraLight=-100`/`350` to confirm no RGB brightening/darkening
- save/load a lit map and compare pre/post lighting grid/profile keys

## Architectural Decisions

- Keep lighting outside `sim/`. This follows the project layering rule and keeps deterministic game state separate from render caches.
- Use an adapter period instead of changing `SpriteInstance` immediately. This follows existing renderer simplicity while letting the data model become correct.
- Add scoped parser behavior for verified light keys only. This avoids broad silent parse changes in combat/movement/economy systems.
- Store `ExtraLight` as raw signed art data. Do not rename the field until call-site churn is justified by the building-body depth patch.
- Defer `BuildingLightClass` spotlights and byte-exact ConvertClass palette tables. They are separate systems and not required for ordinary ambience/lamp parity.

Tech debt introduced:

- Temporary compatibility tint accessors. These should be removed or narrowed once render consumers have migrated to explicit `CellLight` accessors.
- CPU-side LightConvert profile adaptation is not byte-exact palette-table generation. It should be replaced only after the ConvertClass palette-table investigation is done.

## Alternatives Considered

### Big-Bang Renderer ABI Replacement

Replace `SpriteInstance.tint` with profile/scalar fields and update all WGSL/render paths immediately.

Rejected for now because `SpriteInstance.tint` is used by many unrelated render categories. This would create a large blast radius before the data model has tests.

### CPU-Only Tint Fixes Without Render Migration

Fix `ExtraLight`, defaults, parser semantics, and point-light math, but keep `[f32;3]` as the permanent model.

Rejected because it leaves known scalar-consumer parity holes. It is useful as an intermediate state, not as the full architecture.

### Dynamic Lighting First

Implement dirty-cell invalidation and runtime light toggles before the static model.

Rejected because current static math and data parsing are wrong. Dynamic invalidation should use the corrected `CellLightGrid`, not the current tint grid.
