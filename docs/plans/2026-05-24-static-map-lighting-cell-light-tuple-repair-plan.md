# Static Map Lighting Cell-Light Tuple Repair Implementation Plan

> Execute task-by-task. This plan is scoped to static/load-time ordinary map
> lighting only. Do not broaden it into live dirty scheduling, power/detail
> lifecycle updates, superweapon lighting, transient flashes, spotlights, or
> byte-exact LightConvert palette-table generation.

## Goal

Repair Rust's static map-lighting foundation so ordinary cells, raised cells,
static building lamps, terrain tiles, and terrain objects preserve the verified
gamemd cell-light tuple instead of collapsing everything into one RGB tint.

## References

- Design doc: `docs/plans/2026-05-24-static-map-lighting-cell-light-tuple-repair-design.md`
- Contract: `docs/contracts/2026-05-24-static-map-lighting-cell-light-parity-implementation-contract.md`
- Static trace reports:
  - `docs/research/traces/STATIC_LIGHTING_DEFAULT_MAP_FLAT_NO_LAMPS_TRACE.md`
  - `docs/research/traces/STATIC_BUILDING_POINT_LIGHT_RADIUS_FALLOFF_TRACE.md`
  - `docs/research/traces/STATIC_LIGHT_INTENSITY_ZERO_ALLOCATION_GATE_TRACE.md`
  - `docs/research/traces/STATIC_TERRAIN_LIGHT_KEYS_NON_EMITTER_TRACE.md`
  - `docs/research/traces/STATIC_TERRAIN_OBJECT_LIGHT_CONSUMER_BOUNDARY_TRACE.md`
- Core Ghidra reports:
  - `docs/research/MAP_LIGHTING_CELL_COMPUTE_00484180_GHIDRA_REPORT.md`
  - `docs/research/LIGHTCONVERT_NORMALIZE_005558E0_00555AC0_GHIDRA_REPORT.md`
  - `docs/research/LIGHTING_DRAW_CONSUMERS_CELL_FIELDS_GHIDRA_REPORT.md`

## Grounding Summary

Current Rust already routes most visible objects through `CellLightGrid`, but the
grid is not carrying the binary's cell-light bundle. It stores one profile RGB and
one common scalar, then exposes `[f32; 3]` compatibility tints. This causes several
visible mismatches:

- default ordinary flat cells are `0.800` in Rust but `0.950` in gamemd;
- raised cells collapse top/common/bottom into one value;
- building lamps multiply intensity by RGB tint too early, making GALITE much too
  weak;
- terrain tiles receive one uniform ground tint in `app_init.rs`;
- terrain objects do not choose the verified normal vs `SpawnsTiberium=yes` scalar
  branch.

The implementation target is not full palette parity. It is a richer tuple-first
source of truth plus compatibility tint adapters for the current renderer.

## Parity-Critical Items

- Default ordinary flat cell common scalar: `950`.
- Raised sample common/top scalar: `982`; bottom scalar: `1014`.
- Scenario `Ambient`, `Red`, `Green`, `Blue` convert through `field * 1000 / 100`.
- Ordinary branch uses `Ground` and `Level`; special/superweapon lighting is out of scope.
- Scalars clamp to `0..2000`.
- Point-light center is `cell * 256 + 128` leptons.
- Radius test is inclusive; exact edge contributes factor `0`.
- Point-light additive intensity and RGB tint are accumulated separately.
- GALITE center contribution is additive intensity `200` plus RGB `50,50,10`.
- Missing/zero `LightIntensity` creates no source.
- Terrain-object `Light*` keys do not create terrain-owned static emitters.
- Terrain tiles consume common scalar/profile, not a uniform global tint.
- Normal terrain objects use common scalar; `SpawnsTiberium=yes` terrain uses top/alternate scalar.

## File Map

| Action | Path | Responsibility |
| --- | --- | --- |
| Modify | `src/map/lighting.rs` | Cell-light tuple, ordinary scalar math, LightConvert-style normalization/key, point-light accumulation, compatibility accessors, focused tests |
| Modify | `src/app_init.rs` | Stop baking uniform terrain tint; keep lighting grid rebuild as app/render state |
| Modify | `src/map/terrain.rs` | Terrain instance generation uses per-cell lighting grid |
| Modify | `src/app_render/build_instances.rs` | Pass `AppState.lighting_grid` into terrain instance generation |
| Modify | `src/app_instances/overlays.rs` | Terrain-object branch-specific lighting lookup |
| Inspect/modify if needed | `src/rules/terrain_object_type.rs` | Ensure render path can identify `SpawnsTiberium=yes` terrain object types |
| Inspect | `src/app_instances/shp.rs`, `src/app_instances/units.rs`, `src/app_instances/bridges.rs` | Keep compatibility accessors compiling; do not migrate these in this pass unless required |

## Interface Changes

- `CellLight` gains internal-unit tuple fields:
  - `scale16`
  - `additive_intensity`
  - `rgb_key`
  - `top_scalar`
  - `common_scalar`
  - `bottom_scalar`
- `CellLightGrid` gains explicit branch-aware accessors for terrain tile and terrain-object consumers.
- Existing `*_tint_at` compatibility methods remain for current render consumers.
- `terrain::build_visible_instances` gains access to `&CellLightGrid` or an equivalent per-cell lighting callback.

## Task 1 - Baseline And Guardrails

Files to inspect:

- `src/map/lighting.rs`
- `src/app_init.rs`
- `src/map/terrain.rs`
- `src/app_render/build_instances.rs`
- `src/app_instances/overlays.rs`
- `src/rules/terrain_object_type.rs`

Actions:

- Record current lighting tests in `src/map/lighting.rs`; mark tests that encode known mismatches such as default `0.800` or uniform terrain tint.
- Confirm the current dirty worktree state before edits and ignore unrelated changes.
- Run focused baseline tests if the workspace is currently buildable.

Verification:

- Prefer `cargo test map::lighting --lib`.
- If unrelated local changes break the build, record the failure and continue only with focused compile reasoning.

## Task 2 - Introduce Tuple-First `CellLight`

Files:

- `src/map/lighting.rs`

Actions:

- Add named internal-unit constants:
  - `LIGHT_UNIT = 1000`
  - `LIGHT_CLAMP_MIN = 0`
  - `LIGHT_CLAMP_MAX = 2000`
  - `LIGHT_SCALE16_IDENTITY = 0x10000`
  - `BOTTOM_LEVEL_OFFSET = 4`
- Change `CellLight` to store:
  - `profile_id`
  - `rgb_key`
  - `scale16`
  - `additive_intensity`
  - `top_scalar`
  - `common_scalar`
  - `bottom_scalar`
- Keep temporary constructors/accessors so existing call sites still compile.
- Keep compatibility tint generation private to `CellLightGrid`, not as the canonical data model.

Acceptance:

- Existing render call sites can still request `[f32; 3]` compatibility tints.
- Unit tests can inspect the raw scalar tuple directly.

Verification:

- `cargo test map::lighting --lib`

## Task 3 - Repair Ordinary Base Scalar Math

Files:

- `src/map/lighting.rs`

Actions:

- Convert `[Lighting]` values to internal units before computation:
  - Rust parsed `Ambient`, `Red`, `Green`, `Blue` values are already public
    ratio floats, so parsed `1.0` must become internal `1000`.
  - Rust parsed `Ground` and `Level` values use the public map ratio, but
    gamemd's ordinary scalar formula uses the scenario percent fields. For the
    verified defaults this means parsed `Ground=0.20` becomes internal `50`,
    and parsed `Level=0.032` becomes internal `8`.
  - Do not literally apply `value * 1000 / 100` to Rust's parsed `1.0` floats;
    that formula describes gamemd's already-percent scenario fields.
- Build base cell-light tuples from heights:
  - `common/top = ambient + level * z - ground`
  - `bottom = ambient + level * (z + 4) - ground`
- Preserve the verified clamp/order contract: top/common high clamp before
  normalization, RGB normalization produces `scale16`, bottom is scaled by
  `(bottom * scale16) >> 16`, then high/low scalar clamps finish.
- Update or replace tests that currently assert default `0.800`.

Acceptance:

- Default flat no-lamp cell common scalar is `950`.
- Traced raised sample common/top is `982`.
- Traced raised sample bottom is `1014`.

Verification:

- `cargo test default_flat_no_lamps_ground_scalar_is_950 --lib`
- `cargo test default_raised_no_lamps_common_and_bottom_scalars_match_gamemd --lib`
- `cargo test map::lighting --lib`

## Task 4 - Implement Verified Normalization/Profile-Key Bridge

Files:

- `src/map/lighting.rs`

Actions:

- Implement the verified `0x005558E0` helper behavior needed before cache/profile lookup:
  - low-clamp RGB to `0`;
  - high-clamp RGB to `2000`;
  - neutral `1000,1000,1000` keeps `scale16 = 0x10000`;
  - non-neutral RGB computes `scale16`, normalizes max channel to `1000`, and scales additive intensity;
  - preserve deterministic max-channel tie behavior where practical from the report.
- Implement the static/default high-detail key quantization path from `0x00555AC0`:
  - clamp key RGB to `0..1000`;
  - default high-detail mask path uses multiples of `32`.
- Do not implement full LightConvert palette-table generation.
- Apply the verified post-normalization bottom path when building the final
  tuple: `bottom_scalar = (raw_bottom * scale16) >> 16` before final clamping.

Acceptance:

- A focused test proves `scale16`, normalized additive intensity, and `rgb_key` are preserved independently.
- Neutral RGB keeps identity scale and neutral key.

Verification:

- `cargo test lightconvert_normalization_preserves_scale16_and_rgb_key --lib`
- `cargo test map::lighting --lib`

## Task 5 - Repair Point-Light Accumulation

Files:

- `src/map/lighting.rs`

Actions:

- Keep `point_light_from_object` gate: zero converted `LightIntensity` returns `None`.
- Preserve source fields as internal units:
  - `LightIntensity=0.2` -> `200`
  - `LightRedTint=0.05` -> `50`
  - `LightGreenTint=0.05` -> `50`
  - `LightBlueTint=0.01` -> `10`
- In `accumulate_point_lights`, compute lepton-center distance with integer math.
- Apply inclusive radius test.
- Compute falloff factor in milli-units.
- Add intensity to additive accumulator and RGB tint fields to RGB accumulators separately.
- Run normalization/profile update after summing all sources for the cell.
- Remove the current `intensity * tint[channel]` RGB-collapse path.

Acceptance:

- GALITE center contribution preserves additive intensity `200` and RGB `50,50,10`.
- Offset samples preserve traced radius/falloff behavior.
- Zero-intensity buildings still allocate no point light.

Verification:

- `cargo test galite_source_fields_match_rulesmd_units --lib`
- `cargo test galite_point_light_contribution_separates_intensity_and_rgb --lib`
- `cargo test light_intensity_zero_allocates_no_static_source --lib`
- `cargo test map::lighting --lib`

## Task 6 - Move Terrain Tiles To Per-Cell Lighting

Files:

- `src/app_init.rs`
- `src/map/terrain.rs`
- `src/app_render/build_instances.rs`

Actions:

- Remove the map-load loop in `app_init.rs` that writes one `terrain_tint` into every `TerrainCell`.
- Change terrain instance generation so each visible terrain cell derives tint from `CellLightGrid` using the cell's `(rx, ry)`.
- Prefer passing `&CellLightGrid` into `terrain::build_visible_instances`; if that creates an awkward dependency, pass a small callback instead.
- Keep `TerrainCell.tint` only if it is still needed by tests or temporary compatibility; otherwise remove it in the same task.

Acceptance:

- Terrain tile instances for flat and raised cells can receive different lighting inputs under default lighting.
- Terrain tiles use the same `CellLightGrid` as object/render consumers.

Verification:

- Add/update `terrain_tile_instances_consume_per_cell_lighting`.
- Run `cargo test terrain_tile_instances_consume_per_cell_lighting --lib`.
- Run `cargo test map::terrain --lib` if module targeting works; otherwise run the smallest terrain/render test group available.

## Task 7 - Add Terrain-Object Branch-Specific Lighting

Files:

- `src/app_instances/overlays.rs`
- `src/map/lighting.rs`
- `src/rules/terrain_object_type.rs` or existing rules registry access

Actions:

- Add a branch enum or explicit methods in `CellLightGrid` for terrain-object lighting:
  - normal terrain object -> common scalar;
  - `SpawnsTiberium=yes` terrain object -> top/alternate scalar.
- In terrain-object instance generation, resolve the object's terrain type and select the correct branch.
- Do not parse or use terrain-object `Light*` keys as emitters.
- If type data is unavailable, fall back to the normal/common branch.

Acceptance:

- TIBTRE-style objects use the alternate/top scalar branch.
- Normal terrain objects use the common scalar branch.
- Terrain objects still create zero static point lights from `Light*` keys.

Verification:

- Add/update `terrain_object_instances_choose_branch_specific_cell_light`.
- Add/update `terrain_light_keys_do_not_create_emitters`.
- Run focused overlay/lighting tests.

## Task 8 - Compatibility Consumer Sweep

Files:

- `src/app_instances/shp.rs`
- `src/app_instances/units.rs`
- `src/app_instances/bridges.rs`
- `src/app_instances/overlays.rs`
- `src/map/lighting.rs`

Actions:

- Ensure existing `techno_tint_at`, `unit_tint_at`, `infantry_tint_at`, `building_body_tint_at`, `anim_tint_at`, and `bridge_body_tint_at` still compile and derive from the tuple.
- Add comments only where needed to identify compatibility accessors as temporary bridges.
- Do not migrate every consumer to exact scalar branches in this pass unless required by static terrain/terrain-object acceptance.

Acceptance:

- Existing entity, bridge, animation, and overlay instance builders compile.
- No consumer directly reconstructs lighting math outside `CellLightGrid`.

Verification:

- `cargo check`
- Focused instance-builder tests if present.

## Task 9 - Final Static Lighting Validation

Actions:

- Run the focused lighting and terrain tests.
- Run `cargo check`.
- If the app can run locally, perform a smoke check on a map with:
  - flat terrain with no lamps;
  - raised terrain;
  - one GALITE/INGALITE-style lamp;
  - a TIBTRE object near a lamp.

Expected player-visible outcome:

- Ordinary maps are no longer too dark.
- Raised cells visibly differ from ground cells through the per-cell light grid.
- Building lamps are stronger and closer to gamemd because intensity is not multiplied into RGB tint prematurely.
- Terrain objects follow the verified scalar branch boundary.

Out of scope even after this task:

- byte-exact palette-table output;
- live dirty-cell scheduling;
- power/detail/lifecycle changes;
- superweapon lighting transitions;
- transient combat/spark/spotlight screen-space effects.
