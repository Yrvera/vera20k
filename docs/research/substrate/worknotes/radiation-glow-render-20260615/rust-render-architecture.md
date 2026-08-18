# Radiation Green Glow — Rust Render Architecture Lane

**Date:** 2026-06-15
**Lane:** CURRENT Rust render architecture + where a radiation light hooks.
**Open item:** `SUBSTRATE_OPEN_ITEMS_20260610.md` #4 (render-layer dynamic light/glow).
**Scope:** read-only on code + binary. Authority order binary → Ghidra → docs/research → ini.

---

## TL;DR verdict

**A per-cell additive-light tint path ALREADY EXISTS and is live.** It is the same mechanism
gamemd uses for the radiation glow (`LightSourceClass` contributing to per-cell ambient color
— verified in `RADIATION_EMP_GHIDRA_REPORT.md` §1.11). There is **NO separate "dynamic light"
GPU subsystem** (no additive blend pass, no post-process, no light volumes); instead the engine
bakes a per-cell RGB multiplier (`CellLightGrid`) and folds it into every sprite's per-instance
`tint: vec3f`, which the fragment shader applies as `color.rgb * tint`. The radiation glow is
therefore a **data-feed problem, not a new-pipeline problem**: add a render-side light service
that reads `RadiationState` and accumulates a green additive contribution into the existing
`CellLightGrid` (exactly like `accumulate_point_lights` already does for building lamps).

The remaining infra gap is small and well-bounded: (1) a per-frame (or per-`RadLightDelay`-step)
rebuild trigger for the lighting grid driven by radiation, since today the grid is rebuilt only
on building placement/removal events; (2) a `RadiationState → PointLight`-style adapter living in
the app/render layer; (3) deciding the tint target — terrain-pass tint is enough for player-visible
parity, sprite tint on top of it is a one-line extension.

---

## 1. How the tactical scene is lit today

### 1.1 The lighting model is a per-cell RGB MULTIPLIER baked at the app layer

- **`src/map/lighting.rs`** owns the whole model. It parses the map `[Lighting]` section
  (`Ambient/Red/Green/Blue/Ground/Level`) into a `LightingConfig` and builds a
  **`CellLightGrid`** — a `HashMap<(u16,u16), CellLight>` keyed by cell, backed by a
  `LightProfileCache` (a LightConvert-style RGB-identity dedup cache).
- Per cell the model computes integer scalars (`top_scalar`, `common_scalar`, `bottom_scalar`,
  `scale16`, additive intensity) in `1000 == 1.0` units, clamped `[0, 2000]` (cap 2.0), and a
  normalized RGB key. Accessors like `terrain_tile_tint_at()` / `techno_tint_at()` /
  `overlay_tint_at()` collapse that to a final `[f32;3]` tint (`tint_for_light_scalar`,
  lighting.rs:364).
- **Point lights** (building lamps with `LightVisibility`/`LightIntensity`) are accumulated into
  the grid by `accumulate_point_lights()` (lighting.rs:574): signed linear falloff
  `(radius − dist)/radius × intensity`, summed into each cell's raw RGB + additive accumulators,
  then one final clamp/normalize per channel. **This is the exact shape the radiation glow needs**
  — radiation already exposes `radius_leptons`, a per-cell falloff, and a tint color.

This module header explicitly states: "Ordinary map lighting is app/render-derived state;
simulation does not own this data." That is the #1-invariant-compliant home for the glow.

### 1.2 The per-cell tint reaches the GPU as a per-instance vertex attribute (the live seam)

- **`SpriteInstance`** (`src/render/batch.rs:42`) carries `pub tint: [f32; 3]` — documented as
  "RGB color tint from map lighting. [1,1,1] = no tint … up to 2.0 cap from the lighting formula."
- **`batch_shader.wgsl`** (the SHP/terrain path): vertex passes `tint` through; fragment does
  `return vec4f(color.rgb * input.tint, color.a * input.alpha);` (batch_shader.wgsl:85). So the
  tint is a straight per-pixel multiply on the sampled texel — VERIFIED.
- The voxel path (`sprite_voxel_shader.wgsl`) has the same `tint` attribute (location 5) and an
  additional FX stage; units therefore honor the same per-cell tint.
- **Terrain** consumes it in `terrain::build_visible_instances` (terrain.rs:840): per visible
  cell `let tint = lighting_grid.terrain_tile_tint_at((cell.rx, cell.ry))` is written straight
  into the `SpriteInstance.tint`. VERIFIED at terrain.rs:838-856.
- **Entities/overlays** look up the same grid through `app_instances::*` (units.rs, shp.rs,
  overlays.rs, bridges.rs all reference `lighting_grid`) using the category accessors
  (`unit_tint_at`, `overlay_tint_at`, etc.).

**Verdict: every visible tactical sprite is already tinted per-cell by a single shared
`CellLightGrid`.** A green contribution added to that grid would tint terrain AND the units/
buildings standing on irradiated cells, with zero shader or pipeline change.

### 1.3 There is NO other dynamic-light infrastructure

Confirmed by reading `render/mod.rs` (module inventory) and all 7 `.wgsl` shaders:
- `batch_shader.wgsl` — terrain/SHP, multiplicative tint only.
- `zdepth_shader.wgsl` — terrain with per-pixel frag_depth (cliff occlusion).
- `sprite_voxel_shader.wgsl` — voxel byte→palette→FX, tint multiply.
- `shroud_multiply.wgsl` — separate fog/shroud multiply pass (a *different* per-cell modifier).
- `vxl_*.wgsl`, `upscale_catmull_rom.wgsl` — voxel rasterization + final upscale.

No additive-blend light pass, no light-accumulation buffer, no deferred-style light volume, no
screen-space post-process for lighting. (`COMBAT_LIGHT_SPAWN_…GHIDRA_REPORT.md` notes gamemd's
*bright combat impact flash* is a transient screen-space light — that is a separate, still-unbuilt
surface and NOT what radiation uses; radiation uses the persistent per-cell `LightSourceClass`
ambient path, which we already model.)

---

## 2. How terrain vs. sprites get their color (palette / depth / atlas)

### 2.1 Terrain tiles
- Color: TMP tiles are pre-converted to RGBA in the **tile atlas** (`tile_atlas.rs`), sampled as
  an `Rgba8UnormSrgb` texture (nearest). The per-cell light tint is the only runtime color
  modifier on terrain.
- Depth: **only terrain writes depth.** Terrain uses the `pipeline` (Depth32Float, write ON,
  `Less`) for flat tiles and the `zdepth_pipeline` (per-pixel `frag_depth` from a parallel R8
  depth atlas, `Less`) for tiles with embedded Z-data (cliff faces). Cliff redraw re-emits the
  same tile after sprites via `overlay_pipeline` (write ON, `LessEqual`). VERIFIED batch.rs
  pipeline descriptors.

### 2.2 Unit / building / overlay sprites
- VXL units: rendered offline by the software voxel rasterizer to an **R8Uint palette-index
  atlas** (`unit_atlas.rs`, `create_unit_atlas_texture`), then `sprite_voxel_shader.wgsl` does
  byte → palette / per-house ramp (the `house_color_idx` row, bind group 2 = `PaletteSet`) → FX
  → `× tint`. SHP buildings/infantry live in the multi-page RGBA `sprite_atlas` and draw through
  `batch_shader.wgsl`.
- Depth: sprites use **passthrough** (`overlay_passthrough_pipeline`: compare `Always`, write OFF)
  or the voxel pipeline (`LessEqual`, write OFF). Sprites never write depth; they Y-sort on the
  CPU (`sort_by_depth_desc`, build_instances.rs:806) and paint over terrain. This is the
  memory-noted invariant ("only terrain writes depth; sprites use passthrough") and it means a
  radiation glow folded into the tint needs no depth-state changes.

---

## 3. The concrete composite seam (recommended) + alternatives

### SEAM A — accumulate radiation into the existing `CellLightGrid` (RECOMMENDED)

Mirror `accumulate_point_lights`: add a render/app-layer function that walks `RadiationState`
and adds a green additive contribution per affected cell into the grid before the tint accessors
read it. This is the highest-parity, lowest-risk seam because:

- It reproduces gamemd's actual mechanism (radiation = a `LightSourceClass` feeding per-cell
  ambient color — `RADIATION_EMP_GHIDRA_REPORT.md` §1.11), so it satisfies "model the gamemd
  primitive, don't approximate."
- It tints terrain AND the units/buildings/overlays on irradiated cells in one shot (shared grid),
  matching the original where the ambient color affects everything drawn in those cells.
- **Zero GPU change** — no new pipeline, bind group, buffer, or shader. The `tint` attribute and
  the `× tint` multiply already exist.

Two formula details the sim already hands us (study §2.6 / item #4 anchor): intensity
`min(level × RadLightFactor, 2000)`, color = `RadColor × RadTintFactor × (remaining/duration)`,
stepped on `RadLightDelay`. `RadiationState` exposes `sites()` (per-site level/remaining/duration/
radius), `iter_cells()` (per-cell raw level), `current_site_level()`, and `site_at()` — everything
the contribution needs. The light is **green additive** (default `RadColor=0,255,0`), and the
existing accumulator already sums signed RGB+intensity before one clamp, so green tint over a
0.95 base ground naturally reads as a green glow.

**Cost:** small. One adapter fn (~`PointLight`-shaped, or a direct grid accumulation), plus a
rebuild trigger (see §4). Risk: the only subtlety is the additive-vs-multiplicative semantics —
`accumulate_point_lights` is the verified additive path, so route through it rather than
overwriting the multiplicative tint.

### SEAM B — separate additive glow render pass
A dedicated green-additive quad pass per irradiated cell, blended (`One, One`) over the scene
after terrain. Feasible (the batch renderer can host another pipeline), but it **duplicates** the
lighting model gamemd folds into ambient, risks double-counting with the cap, and would not match
how the original composites (which is into the per-cell ambient, not a screen overlay). More GPU
work, lower parity. Not recommended unless SEAM A proves to have a frame-cadence problem.

### SEAM C — per-sprite tint only (no terrain tint)
Tint just the entities standing in radiation. Wrong: in gamemd the ground itself glows green
(the cell ambient changes), so terrain MUST be tinted. SEAM C alone is a parity miss. (It is
already subsumed by SEAM A for free.)

### SEAM D — post-process
Out of scope; no per-cell post-process exists and it would be the least faithful.

---

## 4. The actual infra gap (what must be built)

The tint *consumption* path is 100% built. The gap is the **feed + update cadence**:

1. **Rebuild trigger.** Today `state.lighting_grid` is rebuilt only by
   `rebuild_lighting_grid_from_sim` (`app_init.rs:171`) on map load (app_init.rs:1069),
   building placement/removal (app_input.rs:827), and transitions (app_transitions.rs:151) —
   i.e. **event-driven, not per-frame.** Radiation decays every tick and re-steps on
   `RadLightDelay`, so the grid must be refreshed when radiation is non-empty. Cleanest fit:
   recompute the radiation contribution each frame (or each `RadLightDelay` step) on top of the
   cached base+building grid, gated by `!sim.radiation.is_empty()` so idle matches pay nothing.
   This is the one genuinely new piece of plumbing.

2. **Radiation→light adapter (render/app layer).** A function that reads `RadiationState`
   (`sites()` / `iter_cells()`) and produces the green contribution, applying the
   `intensity = min(level × RadLightFactor, 2000)` and `tint × (remaining/duration)` formulas.
   It belongs next to `collect_live_building_lights` / `rebuild_lighting_grid_from_sim` in the
   app layer (or a new `render`-side light service). It must NOT live in `sim/`.

3. **Rules plumbing.** `RadiationRules` already carries `light_factor`, `tint_factor`, `color`,
   `light_delay` (radiation.rs test fixture confirms the fields parse). Confirm those reach the
   adapter (they live on `rules.radiation`, already threaded into the sim tick at
   world/mod.rs:2305).

No new GPU resource, pipeline, bind group, shader, atlas, or instance-buffer change is required.

---

## 5. #1-invariant compliance (sim never depends on render)

The design keeps the invariant intact by construction:

- `RadiationState` is **pure sim data** — it is serialized, state-hashed (deterministic
  `iter_cells()` ordering exists specifically "for state hashing and the render glow layer",
  radiation.rs:168), and evolves only inside `World::advance_tick` (`apply_detonation` /
  `tick_decay`, world/mod.rs:2299-2305). It knows nothing about lights, tint, or RGB.
- The light service lives in **app/render** and READS `RadiationState` (a `&` borrow) the same
  way `collect_live_building_lights` reads `sim.entities()`. It writes only into the app-owned
  `CellLightGrid` (`AppState.lighting_grid`), which `render/` already consumes.
- Direction of dependency: **render → sim (read-only)**, never the reverse. Radiation tint is a
  pure function of serialized sim state, so it is replay/lockstep-safe and never feeds back into
  the deterministic hash. `render/mod.rs` already declares "render/ may READ from sim/ … NEVER
  mutates sim state" — this design stays inside that contract.

---

## 6. Evidence ledger

| Claim | Status | Evidence |
|---|---|---|
| Per-cell tint is a per-instance `vec3f` multiplied in the fragment shader | VERIFIED (code) | `render/batch.rs:42,54`; `batch_shader.wgsl:85` |
| Terrain consumes `lighting_grid.terrain_tile_tint_at()` into `SpriteInstance.tint` | VERIFIED (code) | `map/terrain.rs:746,840-856` |
| Point lights accumulate via signed linear falloff, summed before one clamp | VERIFIED (code) | `map/lighting.rs:574-620` |
| Lighting grid rebuilt event-driven (load/placement/transition), not per-frame | VERIFIED (code) | `app_init.rs:171,1069`; `app_input.rs:827`; `app_transitions.rs:151` |
| No additive/post-process/light-volume pipeline exists | VERIFIED (code) | `render/mod.rs` inventory; all 7 `.wgsl` files |
| Only terrain writes depth; sprites passthrough/LessEqual no-write | VERIFIED (code) | `render/batch.rs` pipeline descriptors |
| `RadiationState` exposes `sites()`, `iter_cells()`, `current_site_level()` for render | VERIFIED (code) | `sim/radiation.rs:162-185` |
| Radiation ticks inside sim, never touches render | VERIFIED (code) | `sim/world/mod.rs:2299-2305` |
| gamemd radiation glow = `LightSourceClass` feeding per-cell ambient color, RadColor×RadTintFactor, intensity RadLevel×RadLightFactor, fades over lifetime, updates every RadLightDelay | VERIFIED (doc, ghidra-sourced) | `RADIATION_EMP_GHIDRA_REPORT.md` §1.11 (RadSite activation `0x0065B580`, §1.6) |
| gamemd LightSource contributes to per-cell ambient (not consumed by DrawBody directly) | VERIFIED (doc, ghidra-sourced) | `BUILDINGCLASS_DRAWBODY_GHIDRA_REPORT.md` §10 |
| Radiation is NOT SpecialFlags-gated / TS-legacy (active in stock YR Desolator deploy) | INFERRED | `SUBSTRATE_OPEN_ITEMS_20260610.md` #4 ("player-visible on EVERY Desolator deploy"); no SpecialFlags gate found in the radiation report's activation path. Not independently re-verified in Ghidra this lane. |

**UNKNOWN / not verified this lane:** the exact gamemd compositing order of the radiation light
vs. building lights within a cell (additive sum is the model both Rust and the report describe,
but the precise clamp interaction with multiple overlapping RadSites on one cell was not
bit-traced here); whether gamemd updates the light on a per-site `RadLightDelay` step or a global
frame modulo (the damage-side timer is per-site/activation-anchored per radiation.rs, but the
light timer was not separately traced). These are formula-lane / sim-lane questions, not render-
architecture blockers — the render seam (SEAM A) is unaffected by either answer.
