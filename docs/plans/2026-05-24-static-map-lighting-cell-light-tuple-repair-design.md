# Static Map Lighting Cell-Light Tuple Repair Design

## Goal

Replace the current collapsed per-cell RGB tint approximation with a gamemd-style
static cell-light bundle that preserves ordinary map ambience, static building lamp
contribution, and render-consumer scalar branches.

## Architecture Context

Static lighting currently lives outside `sim/`, which is the right boundary. The
current flow is:

```text
map [Lighting] + terrain heights
  -> src/map/lighting.rs::build_cell_light_grid_from_heights
  -> CellLightGrid
  -> app/render instance builders
  -> SpriteInstance.tint
```

`src/map/lighting.rs` already has a `CellLightGrid`, `CellLight`, and
`LightProfileCache`, but the data shape is still a compatibility tint: one RGB
profile multiplied by one common scalar. That loses gamemd's separate additive
intensity, RGB/profile key, and top/common/bottom scalar fields.

Terrain is currently a special mismatch. `src/app_init.rs` computes one
`terrain_tint` from ground lighting and writes it into every `TerrainCell`.
`src/map/terrain.rs::build_visible_instances` later uploads that baked tint. This
prevents raised cells and lamp-affected cells from using per-cell lighting.

Terrain objects currently call
`CellLightGrid::terrain_object_tint_at` from `src/app_instances/overlays.rs`.
That accessor cannot reproduce the binary branch where normal terrain objects use
the common scalar while `SpawnsTiberium=yes` terrain uses the alternate/top scalar.

## Impact Analysis

Primary changes are confined to map/render/app instance generation:

- `src/map/lighting.rs`: define the authoritative static cell-light tuple, ordinary
  scalar math, point-light accumulation, profile normalization key, and temporary
  compatibility tint accessors.
- `src/app_init.rs`: stop baking a uniform terrain tint into `TerrainGrid` as the
  authoritative lighting source.
- `src/map/terrain.rs`: terrain visible-instance building must receive or look up
  `CellLightGrid` so each tile uses the cell's profile/scalar-derived tint.
- `src/app_render/build_instances.rs`: pass the current lighting grid into terrain
  instance generation.
- `src/app_instances/overlays.rs`: terrain-object instance generation must choose
  the correct scalar branch using terrain object type data.
- `src/rules/terrain_object_type.rs` or the rules access path: expose enough
  render-side terrain-object type data to know `SpawnsTiberium=yes`.

Compatibility tint users can remain initially:

- `src/app_instances/shp.rs`
- `src/app_instances/units.rs`
- `src/app_instances/bridges.rs`
- animation and overlay paths not yet covered by this static contract

Risk areas:

- The current renderer takes one `SpriteInstance.tint`. This design cannot be
  final byte-exact LightConvert parity until the palette-table path is researched,
  but it preserves the required source fields instead of throwing them away.
- Changing terrain lighting from baked uniform tint to per-cell lookup will make
  visible output change immediately. That is intended; default maps should become
  less dark and raised/lamp cells should diverge.
- Terrain-object branch selection needs rules/type lookup. Do not add render-only
  dependencies into `sim/`; pass data from existing app/rules state downward.

## Chosen Approach

Use a tuple-first `CellLight` model with temporary compatibility render bridges.

`CellLight` becomes the authoritative static map-lighting payload in internal
integer units. The tuple should preserve:

- 16.16 normalization scale;
- additive point-light intensity;
- normalized/profile RGB key;
- top scalar;
- common scalar;
- bottom scalar;
- optional profile id for current compatibility cache/render lookup.

Point lights add intensity and RGB tint separately. Final compatibility tint is
derived from the tuple for today's renderer, but the tuple remains available for
later LightConvert/palette work.

This approach was chosen over patching `[f32; 3]` because the verified gamemd draw
consumers do not consume one RGB tint. It was chosen over a full LightConvert
render rewrite because byte-exact palette-table output is still a known blocker.

## Tiny-Detail Ledger

- Default ordinary flat cell scalar is `950`, not current Rust `800`.
  Source: `STATIC_LIGHTING_DEFAULT_MAP_FLAT_NO_LAMPS_TRACE.md`, Ghidra `0x00484180`.
- Raised cells need separate common/top and bottom fields; traced raised sample has
  common/top `982` and bottom `1014`.
  Source: `STATIC_LIGHTING_DEFAULT_MAP_FLAT_NO_LAMPS_TRACE.md`, Ghidra `0x00484180`.
- Scenario base fields convert as `field * 1000 / 100`.
  Source: `MAP_LIGHTING_CELL_COMPUTE_00484180_GHIDRA_REPORT.md`.
- Ordinary scalar branch uses `Ground` and `Level`; superweapon/special lighting
  modes are out of scope for this design.
  Source: `MAP_LIGHTING_CELL_COMPUTE_00484180_GHIDRA_REPORT.md`.
- Scalar clamp is `0..2000`.
  Source: `MAP_LIGHTING_CELL_COMPUTE_00484180_GHIDRA_REPORT.md`.
- Point-light cell centers are `cell * 256 + 128` leptons.
  Source: `MAP_LIGHTING_CELL_COMPUTE_00484180_GHIDRA_REPORT.md`.
- Radius inclusion is inclusive; exact edge contributes factor `0`.
  Source: `MAP_LIGHTING_CELL_COMPUTE_00484180_GHIDRA_REPORT.md`.
- Point-light additive intensity and RGB tint are separate accumulators.
  Source: `STATIC_BUILDING_POINT_LIGHT_RADIUS_FALLOFF_TRACE.md`.
- GALITE center source contribution is additive intensity `200` and RGB
  `50,50,10`, not RGB `10,10,2`.
  Source: `STATIC_BUILDING_POINT_LIGHT_RADIUS_FALLOFF_TRACE.md`.
- Zero `LightIntensity` creates no building light source.
  Source: `STATIC_LIGHT_INTENSITY_ZERO_ALLOCATION_GATE_TRACE.md`, Ghidra `0x00440580`.
- Terrain-object `Light*` keys do not create terrain-owned static map lights.
  Source: `STATIC_TERRAIN_LIGHT_KEYS_NON_EMITTER_TRACE.md`.
- TMP terrain consumes a profile/convert concept plus common scalar.
  Source: `LIGHTING_DRAW_CONSUMERS_CELL_FIELDS_GHIDRA_REPORT.md`.
- Terrain objects normally use common scalar, while `SpawnsTiberium=yes` terrain
  uses the alternate/top scalar branch.
  Source: `STATIC_TERRAIN_OBJECT_LIGHT_CONSUMER_BOUNDARY_TRACE.md`.

## Design

### Components

#### `src/map/lighting.rs`

Introduce explicit internal-unit constants and helper names:

- `LIGHT_UNIT = 1000`
- `LIGHT_CLAMP_MIN = 0`
- `LIGHT_CLAMP_MAX = 2000`
- `LEPTONS_PER_CELL = 256`
- `HALF_CELL_LEPTONS = 128`
- `BOTTOM_LEVEL_OFFSET = 4`

Keep `LightingConfig` as the parsed public INI shape for now, but convert to
internal integer units before cell computation. The computation API should avoid
doing visible parity math in floats.

Proposed payload shape:

```rust
pub struct CellLight {
    pub profile_id: LightProfileId,
    pub rgb_key: LightRgbKey,
    pub scale16: i32,
    pub additive_intensity: i32,
    pub top_scalar: i32,
    pub common_scalar: i32,
    pub bottom_scalar: i32,
}
```

The exact field names can change during implementation, but these concepts must
remain separate.

`build_cell_light_grid_from_heights` should:

1. convert `Ambient`, `Red`, `Green`, `Blue`, `Ground`, and `Level` into internal
   units;
2. build a base light tuple per cell using ordinary static branch math;
3. initialize profile/RGB state from scenario RGB;
4. compute top/common and bottom scalar fields separately.

`accumulate_point_lights` should:

1. iterate all existing cells;
2. skip inactive/detail-gated sources according to the currently represented
   static defaults;
3. compute integer falloff from lepton centers;
4. add source intensity into `additive_intensity`;
5. add source RGB fields into the RGB accumulators separately;
6. run the verified `0x005558E0`-style RGB normalization enough to produce
   `scale16`, normalized additive intensity, and normalized RGB channels;
7. quantize/profile the RGB key with the default high-detail mask path currently
   targeted by static load-time rendering;
8. apply scalar clamp order as far as current verified helper evidence supports.

`CellLightGrid` should expose explicit accessors:

- `terrain_tile_light_at(cell)` -> common-scalar compatibility tint/profile input;
- `terrain_object_light_at(cell, branch)` -> top/common branch selection;
- `overlay_light_at(cell, branch)` for future overlay branch migration;
- existing `techno_tint_at`, `unit_tint_at`, `anim_tint_at`, etc. remain as
  temporary compatibility accessors.

#### `src/map/terrain.rs`

Remove the assumption that `TerrainCell.tint` is authoritative. Two migration-safe
options are acceptable:

1. pass `&CellLightGrid` into `build_visible_instances` and compute terrain tile
   tint from `cell.rx, cell.ry` at instance-build time;
2. replace `TerrainCell.tint` with a cached light cell reference/key only if that
   fits the existing terrain grid lifecycle cleanly.

The first option is preferred because lighting is already rebuilt from live app
state and terrain geometry is map-load data.

#### `src/app_init.rs`

Stop writing one uniform ground tint into every terrain cell. `TerrainGrid` should
remain geometry/art data. Static lighting is already stored as `AppState.lighting_grid`.

#### `src/app_render/build_instances.rs`

Pass `&state.lighting_grid` into terrain instance generation. This keeps terrain
tile lighting synced with the same grid used by objects.

#### `src/app_instances/overlays.rs`

For terrain objects, resolve type data from `state.rules.terrain_object_types` or
the existing equivalent registry. Choose:

- common scalar branch for normal terrain objects;
- top/alternate scalar branch for `SpawnsTiberium=yes`.

Do not create point lights from terrain object `Light*` keys.

### Interfaces / Contracts

`CellLightGrid` should become the only app-facing source for static per-cell
lighting. Render code should not recompute map `[Lighting]` values.

Compatibility tint accessors are allowed only as adapters:

```rust
fn tint_for_common_scalar(&self, cell: (u16, u16)) -> [f32; 3]
fn tint_for_top_scalar(&self, cell: (u16, u16)) -> [f32; 3]
fn tint_for_bottom_scalar(&self, cell: (u16, u16)) -> [f32; 3]
```

The implementation should make branch choice explicit at call sites where binary
evidence already exists, rather than hiding every consumer behind
`tint_or_default`.

The compatibility adapter must not become the source of truth. It is allowed to
multiply a normalized/profile RGB approximation by the selected scalar for today's
`SpriteInstance.tint`, but the stored tuple must retain `scale16`,
`additive_intensity`, `rgb_key`, `top_scalar`, `common_scalar`, and
`bottom_scalar` independently.

### Data Flow

```text
Map INI [Lighting]
  -> LightingConfig
  -> internal scenario light units
  -> base CellLightGrid from terrain heights
  -> static/live building PointLight collection
  -> point-light accumulation into separate additive/RGB fields
  -> profile/key normalization + scalar fields
  -> terrain/terrain-object/render compatibility accessors
  -> SpriteInstance.tint for current renderer
```

The current renderer still receives `[f32; 3]` tints, but those tints must be
derived from the richer tuple. That means later LightConvert work can replace the
adapter without re-researching core cell-light computation.

### Error Handling

Lighting parsing should keep current fallback behavior: missing `[Lighting]` or
missing keys use verified defaults. Invalid numeric strings should continue to use
the parser family's existing fallback behavior unless a separate parser contract
requires stricter Westwood-specific handling.

No new runtime errors are needed for normal missing data. If terrain-object type
data is missing, use the normal/common branch and keep a debug log only if the
codebase already logs similar missing art/rules data.

### Testing Strategy

Add focused unit tests in `src/map/lighting.rs` first:

- `default_flat_no_lamps_ground_scalar_is_950`
- `default_raised_no_lamps_common_and_bottom_scalars_match_gamemd`
- `galite_source_fields_match_rulesmd_units`
- `galite_point_light_contribution_separates_intensity_and_rgb`
- `lightconvert_normalization_preserves_scale16_and_rgb_key`
- `light_intensity_zero_allocates_no_static_source`
- `terrain_light_keys_do_not_create_emitters`

Add render-facing or instance-builder tests after the data model is in place:

- `terrain_tile_instances_consume_per_cell_lighting`
- `terrain_object_instances_choose_branch_specific_cell_light`

Existing tests that assert `0.800` default tint or uniform `terrain_tint` should be
updated or removed because they encode the current mismatch.

## Architectural Decisions

- Keep static lighting outside `sim/`. The grid is derived app/render state and does
  not belong in deterministic simulation state.
- Prefer integer internal units for parity-sensitive computation. Floats are only a
  compatibility conversion for current GPU tint upload.
- Preserve `CellLightGrid` as the central API instead of inventing a separate render
  lighting service. The current code already routes all relevant app consumers
  through this grid.
- Defer byte-exact `LightConvert` palette table generation. The tuple/profile shape
  is designed to carry the data needed for that later work without guessing it now.
- Defer live dirty scheduling, power/detail/lifecycle gates, superweapon lighting,
  and transient screen-space effects. They are real systems but outside the accepted
  static/load-time scope.

## Alternatives Considered

### A. Patch Existing Tint Math

This would adjust formulas while keeping `[f32; 3]` as the main model. It is lower
churn but cannot preserve additive intensity, separate RGB profile keys, or
top/common/bottom branch selection. It leaves verified player-visible parity holes.

### C. Full LightConvert Render Pipeline Now

This would push profile IDs and scalar fields into the renderer immediately. It is
closer to the final architecture, but exact palette-table output remains blocked on
additional LightConvert research. Implementing it now would require guesswork.

## Handoff

If this design is accepted, the next step is an implementation plan for Approach B:

1. update `CellLight`/`CellLightGrid` tuple model and tests;
2. repair ordinary scalar math;
3. repair point-light accumulation;
4. add the verified normalization-scale/profile-key bridge currently needed by
   compatibility rendering;
5. migrate terrain tiles to per-cell lookup;
6. migrate terrain-object branch selection;
7. run focused lighting tests and any render smoke checks available.
