# PixelFX Water/Ore Sparkle Render Module — Design

**Status:** approved 2026-05-18.
**Topic:** stateless, per-frame, per-cell water + ore sparkle render module reproducing gamemd.exe's `DrawPixelFXSparkles @ 0x006D7840`.
**Reference research:** `docs/research/PIXEL_FX_SPARKLES_GHIDRA_REPORT.md` (verified this session, including §14 close-out).

## Goal

Render the per-frame water/ore sparkle effect — a 1-pixel dot drawn over visible water and ore cells, pulsing dim → bright → dim on its own per-cell cycle — so the player observation matches gamemd.exe within the project parity bar (visually indistinguishable in a single skirmish; not bit-identical pixels).

## Architecture Context

Render flow is a 6-phase pipeline in [src/app_render/mod.rs:42-114](../../src/app_render/mod.rs#L42-L114):
1. Build world instances (terrain, overlays, units)
2. Build debug instances
3. Update minimap + build UI instances
4. Build sidebar instances
4b. Rebuild shroud A-buffer
5. Upload all instance vectors to GPU pool
6. Dispatch draw calls in render order (in [src/app_render/draw_passes.rs](../../src/app_render/draw_passes.rs))

Draw order in `dispatch_draw_passes`:
- Step 1: Terrain (zdepth pipeline)
- Step 2: Bridge body (zdepth)
- Step 3: Overlays (passthrough)
- Step 3.5: Smudges
- Step 4: Bridge entities (merge)
- Step 5: Ground objects (units/buildings, Y-merge)
- Step 6: Building turrets
- Step 7: Cliff redraw (zdepth, depth-tested)
- Step 8: Debug
- Step 9: Shroud/fog
- Step 10: UI/sidebar

The new sparkle pass slots in as **Step 5.5** — between ground objects and turrets — matching gamemd's draw position (between `Tactical_ObjectRenderingLoop` and the UI pass).

Per-cell state APIs already available (read-only):
- `ResolvedTerrainCell` at [src/map/resolved_terrain.rs:73-189](../../src/map/resolved_terrain.rs#L73-L189) — has `is_water: bool`, `has_bridge_deck: bool` (static).
- `Simulation.fog.is_cell_visible(owner_id, rx, ry) -> bool` at [src/sim/vision/mod.rs:243-252](../../src/sim/vision/mod.rs#L243-L252) — current-sight check.
- `Simulation.bridge_state.is_bridge_walkable(rx, ry) -> bool` at [src/sim/bridge_state/mod.rs:919-940](../../src/sim/bridge_state/mod.rs#L919-L940) — runtime bridge-deck check (handles collapse).
- `Simulation.overlays.cell(rx, ry).overlay_id` + `registry.flags(id).tiberium` — ore detection.
- `Simulation.occupancy.get(rx, ry).occupants.is_empty()` at [src/sim/occupancy.rs:217-229](../../src/sim/occupancy.rs#L217-L229) — occupancy check.
- `World.tick: u64` at [src/sim/world/mod.rs:76](../../src/sim/world/mod.rs#L76) — canonical sim tick counter.

Existing instance batch infrastructure:
- `SpriteInstance` at [src/render/batch.rs:42-75](../../src/render/batch.rs#L42-L75) — fields include `position`, `size`, `uv_origin`, `uv_size`, `depth`, `tint`, `alpha`. Per-instance tint is exactly the color channel we need.
- `SelectionOverlay::white_texture()` at [src/render/selection_overlay.rs:319](../../src/render/selection_overlay.rs#L319) — already-built 1×1 white `BatchTexture` reusable for tinted quads.
- `InstanceBufferPool` — keyed by string name; `pool.upload(...)` from build phase, `pool.get(...)` in dispatch phase.

## Impact Analysis

**Files touched:**
- NEW: `src/render/pixel_fx_sparkles.rs` (~200 lines including tests)
- MOD: `src/render/mod.rs` (1 line: `pub mod pixel_fx_sparkles;`)
- MOD: `src/util/config.rs` (~5 lines: `extra_animations: bool` field on `GraphicsConfig`, default true)
- MOD: `src/app_render/build_instances/mod.rs` (~25 lines: add `cell_sparkles` field to `WorldInstances`, call `build_sparkle_instances` in `build_world_instances`)
- MOD: `src/app_render/mod.rs` (~3 lines: `pool.upload(..., "cell_sparkles", &world.cell_sparkles)` in `upload_to_gpu`)
- MOD: `src/app_render/draw_passes.rs` (~10 lines: new Step 5.5 between Step 5 and Step 6)
- MOD: `config.toml.example` (~1 line: `extra_animations = true` under `[graphics]`)

**Reads (no mutations introduced):** `ResolvedTerrainGrid`, `FogGrid`, `OverlayGrid`, `OverlayTypeRegistry`, `OccupancyGrid`, `BridgeRuntimeState`, `GameConfig`, `World.tick`, camera, `SelectionOverlay::white_texture`.

**Risk areas:**
- Hash distribution quality — bad distribution = visible grid patterns or beat-sync rows of cells pulsing together. Mitigate with splitmix64 (well-known good distribution); sanity-check empirically.
- Draw-order placement — inserting at the wrong step (e.g., after turrets) breaks occlusion. Mitigate by placing between Step 5 and Step 6 and confirming with manual visual test.
- Sight-API ownership — `is_cell_visible(owner_id, ...)` requires the local-player `InternedId` at render time; already plumbed for shroud rendering (see [draw_passes.rs:253-256](../../src/app_render/draw_passes.rs#L253-L256)).
- Bridge-deck API source — runtime (`is_bridge_walkable`) vs static (`has_bridge_deck`). Must use runtime so collapsed bridges re-enable sparkles correctly.
- Determinism — stateless hash + `world.tick` gives identical output across clients on the same tick. No lockstep risk.

## Chosen Approach

**Stateless / hash-derived per-cell sparkle, faithful per-cell `LerpSpeed` variability, fixed 2500ms cycle bucket per species.**

Every frame, for each visible viewport cell:
1. Run the 6-condition gate (water OR ore; not occupied; in sight; not under bridge deck; species applies).
2. Compute a per-cell `cell_offset_ms` from `splitmix64(coord_key(rx, ry)) % bucket_ms` to break global beat-sync.
3. Bucket the shifted time into `cycle_index` and `cycle_pos_ms`.
4. Re-hash `splitmix64(coord_key ^ cycle_index)` → bit-extract this cycle's `sub_x`, `sub_y`, `timer_init_ms`, `lerp_speed`, peak-colour noise.
5. Phase logic: if in timer-wait, draw base color; if in active phase, apply ping-pong lerp; else draw base color (cycle's active phase ended before bucket boundary).
6. Emit one `SpriteInstance` with `size=[1,1]`, `tint=current_rgb`, `alpha=1.0`, UV pointing at the 1×1 white texel from `SelectionOverlay`.

**Why this approach** (over the simplified fixed-cycle-per-species version that was Approach B in the brainstorm):
- Per-cycle randomized `LerpSpeed` is what produces gamemd's "scattered drift" — neighbour cells with same offset don't pulse in lockstep over time.
- The simplified version produces a visible artifact (synchronized cell groups) that fails the parity bar.
- ~30 extra lines of math for full parity coverage; trivial cost.

**Acknowledged parity drift** (one item):
- gamemd's sequential cycles tile back-to-back with variable duration. Our stateless approximation re-inits cells on a fixed 2500ms bucket boundary. Cells whose active phase ends before the boundary sit at base color until the next bucket. Estimated player-visibility: imperceptible in single-skirmish play; would show as a slight autocorrelation pattern in a frame-by-frame diff. Accepted; flagged for the pixel-diff audit (option D from the original menu) to confirm.

## Tiny-Detail Ledger

Every item below cites its source. Each is a constraint the implementation must preserve. The unit tests (Testing Strategy section) lock these into source so they can't drift silently.

### Constants — water sparkle parameters (read directly from binary `0x008367C8`)
- L1. `WATER_BASE_RGB = (40, 40, 80)` — dark indigo, lower endpoint of the lerp. [doc §5.2]
- L2. `WATER_PEAK_RGB = (158, 158, 224)` — pale lavender-blue, upper endpoint. [doc §5.2]
- L3. `WATER_COLOR_NOISE_SHIFT = 5` — per-cycle, peak channel R/G/B each gets `noise = rand() & 0x1F` subtracted (0..31 noise per channel). [doc §5.2, §6.1]
- L4. `WATER_LERP_SPEED_RANGE = [3, 12]` per ms per cell, re-rolled each cycle. [doc §5.2]
- L5. `WATER_TIMER_INIT_MASK = 0xFFF` — initial timer randomized 0..4095 ms per cycle. [doc §5.2]

### Constants — ore sparkle parameters (binary `0x008367F0`)
- L6. `ORE_BASE_RGB = (176, 144, 0)` — dark amber/brown. [doc §5.2]
- L7. `ORE_PEAK_RGB = (255, 255, 240)` — near-pure white. [doc §5.2]
- L8. `ORE_COLOR_NOISE_SHIFT = 0` — no peak-color noise on ore. [doc §5.2]
- L9. `ORE_LERP_SPEED_RANGE = [15, 30]` per ms per cell. [doc §5.2]

### Geometry
- L10. Sub-pixel offset X: `[-31, 32]` (6 bits of `rand` minus 0x1F). [doc §6.1]
- L11. Sub-pixel offset Y: `[-15, 16]` (5 bits of `rand` minus 0x0F). [doc §6.1]
- L12. Sparkle is EXACTLY one pixel: `SpriteInstance.size = [1.0, 1.0]`. [doc §4, §6.4]

### Lerp formula
- L13. Phase domain `[0, 0x2000]` per cycle. [doc §3, §6.3]
- L14. Ping-pong: `lerp = phase & 0xFFF; if (phase & 0x1000) { lerp = 0x1000 - lerp; }`. [doc §6.3]
- L15. Color: `current_channel = (base * (0x1000 - lerp) + peak * lerp) >> 12` per channel. [doc §6.3]
- L16. Phase 0 → base color (dim); phase 0x1000 → peak (bright); phase 0x1FFF → near base. Sparkle fades IN then OUT, NOT a sharp blink. [doc §6.3 table]

### Gate conditions (all must pass per cell)
- L17. `LandType == Water` OR `Get_Tiberium_Value > 0`. [doc §4]
- L18. `FirstObject == NULL` (no occupant). [doc §4]
- L19. Cell is currently in sight (gamemd `cell+0x12C bit 0x10`). Operationally: `fog.is_cell_visible(local_owner, rx, ry)`. [doc §4, §14.1]
- L20. Cell is NOT under bridge deck (gamemd `cell+0x140 bit 0x1000`). Operationally: `bridge_state.is_bridge_walkable(rx, ry) == false`. [doc §4, §14.2]
- L21. Cell is NOT shrouded — subsumed by L19 since shrouded ⇒ not visible. [doc §4]
- L22. `g_ExtraAnimationsEnabled != 0`. Operationally: `config.graphics.extra_animations == true`. [doc §4, §8]

### Visual subtleties
- L23. Cell STARTS dim on each cycle (current = base, not peak). [doc §3 final paragraph]
- L24. Sub-pixel offset AND peak color noise re-randomize per cycle — sparkle visibly "moves" between cycles. [doc §6.4]
- L25. Most cells dim at any instant; peak is brief mid-cycle (~17% of active duration). Effect reads as "scattered random points of light", NOT uniform glow. [doc §5.3]
- L26. Cells start asynchronously due to randomized initial timer — no global beat. [doc §5.3]

### Draw order
- L27. Drawn AFTER unit/object pass, BEFORE UI/turret pass. Maps to Step 5.5 between Step 5 (ground objects) and Step 6 (turrets) in `draw_passes.rs`. [doc §2]
- L28. Opaque overwrite (no alpha blending in gamemd, no Z-test). In wgpu: `alpha = 1.0`, use existing sprite pipeline's blend state (verify in unit test that it's effectively opaque-overwrite at the cell's screen-Y depth). [doc §6.4]

### Cycle frequency (derived from L4, L9)
- L29. Water cycle active duration: `0x2000 / lerp_speed` ms ≈ 170..680 ms per cycle. Total cycle ≈ timer (0..4095 ms) + active (170..680 ms). [doc §5.3]
- L30. Ore cycle: lerp_speed 15..30 → active 280..560 ms. Total ≈ timer + active. Much faster than water. [doc §5.3]

## Design

### Components

One new module at `src/render/pixel_fx_sparkles.rs`:

- `struct SparkleParams` — per-species constants (base, peak, noise bits, lerp speed range).
- `const WATER: SparkleParams` and `const ORE: SparkleParams` — initialized from L1–L9.
- `const WATER_CYCLE_BUCKET_MS: u64 = 2500` and `const ORE_CYCLE_BUCKET_MS: u64 = 2500`.
- `struct SparkleInput<'a>` — borrowed read-only state needed for the build call. Holds:
  - `world_tick: u64`
  - `sim_tick_hz: u32`
  - `enable_extra_animations: bool`
  - `local_owner_id: InternedId`
  - `viewport_cells: &'a [(u16, u16)]`
  - `resolved_terrain: &'a ResolvedTerrainGrid`
  - `overlays: &'a OverlayGrid`
  - `overlay_registry: &'a OverlayTypeRegistry`
  - `occupancy: &'a OccupancyGrid`
  - `fog: &'a FogGrid`
  - `bridge_state: &'a BridgeRuntimeState`
  - `camera: CameraSnapshot`
  - `white_texture_uv: UvRect`
  - `screen_y_for_depth: fn(u16, u16) -> f32` (or similar — match existing pattern)
- `pub fn build_sparkle_instances(input: &SparkleInput<'_>) -> Vec<SpriteInstance>` — the public entry, iterates `viewport_cells`, returns one `SpriteInstance` per qualifying cell.
- `fn compute_sparkle_for_cell(rx, ry, clock_ms, input) -> Option<SpriteInstance>` — gate + math + emit.
- `fn splitmix64(x: u64) -> u64` — inline hash function.
- `fn coord_key(rx: u16, ry: u16) -> u64` — pack cell coords for hashing.
- `fn ping_pong_lerp(phase: u32, base: [u8;3], peak: [u8;3]) -> [u8;3]` — L13–L16.
- `fn lerp_speed_in_range(seed: u64, params: &SparkleParams) -> u32` — bias 4 bits into species range.
- `fn peak_with_noise(seed: u64, params: &SparkleParams) -> [u8;3]` — apply L3 / L8 noise.

### Interfaces / Contracts

The module exposes ONE public function (`build_sparkle_instances`) and ONE public input struct (`SparkleInput`). All other items are private.

Determinism contract: for any `(world_tick, rx, ry, immutable_input)`, `compute_sparkle_for_cell` returns the same `Option<SpriteInstance>`. This is enforced by:
- No interior mutability.
- No global state (no `static mut`, no thread locals).
- No floating-point arithmetic in the math path (`f32` only at the boundary when converting RGB to `SpriteInstance.tint`).
- Hash function and constants compile-time fixed.

### Data Flow

```
phase 1: build_instances
  ├─ build_world_instances(state, vsw, vsh)
  │    ├─ ... (terrain, overlay, etc.)
  │    └─ build_sparkle_instances(&SparkleInput { ... })  ← NEW
  └─ returns WorldInstances { ..., cell_sparkles: Vec<SpriteInstance> }

phase 5: upload_to_gpu
  └─ pool.upload(&gpu, "cell_sparkles", &world.cell_sparkles)  ← NEW

phase 6: dispatch_draw_passes
  ├─ Step 5: ground objects (existing)
  ├─ Step 5.5: cell_sparkles  ← NEW
  │    └─ batch_renderer.draw_sprites_range(
  │         &mut pass,
  │         selection_overlay.white_texture(),
  │         buf, 0, count,
  │       )
  └─ Step 6: turrets (existing)
```

### Error Handling

None needed. The function is pure, total over its input domain. Cells without resolved-terrain entries return `None` from the gate (correct: off-map cells produce no sparkle). No I/O, no allocation failure paths (Vec allocation is unrecoverable at this scale anyway). No `Result` types in the module's public API.

### Testing Strategy

Unit tests (`#[cfg(test)]` block at bottom of `src/render/pixel_fx_sparkles.rs`):

| Test name | Asserts |
|---|---|
| `water_constants_match_report` | All WATER fields match L1–L5 byte-for-byte. |
| `ore_constants_match_report` | All ORE fields match L6–L9 byte-for-byte. |
| `lerp_at_phase_0_is_base` | `ping_pong_lerp(0, base, peak) == base` (L23). |
| `lerp_at_phase_0x1000_is_peak` | `ping_pong_lerp(0x1000, base, peak) == peak`. |
| `lerp_at_phase_0x1FFF_is_near_base` | Within rounding of base. |
| `lerp_ping_pong_symmetry` | `ping_pong_lerp(0x1000 - x, ...) == ping_pong_lerp(0x1000 + x, ...)` for x in 1..0x1000 (L14). |
| `same_tick_same_cell_same_rgb` | Two identical calls produce identical Option<SpriteInstance> — determinism. |
| `cell_offset_breaks_sync` | (rx,ry) and (rx+1,ry) yield DIFFERENT cycle_pos_ms at same world_tick (L26). |
| `gate_skipped_when_not_water_or_ore` | Returns None for clear-ground cell. |
| `gate_skipped_when_occupied` | Returns None for water cell with occupant. |
| `gate_skipped_when_not_visible` | Returns None for water cell where fog.is_cell_visible returns false (L19, L21). |
| `gate_skipped_under_bridge_deck` | Returns None for water cell where bridge_state.is_bridge_walkable returns true (L20). |
| `disabled_by_extra_animations_off` | `build_sparkle_instances` with `enable_extra_animations=false` returns empty Vec, doesn't iterate (L22). |
| `subpos_range_water` | Sample 1000 (cell, cycle) pairs; sub_x ∈ [-31, 32], sub_y ∈ [-15, 16] (L10, L11). |
| `cycle_re_init_changes_sub_pos` | Same cell, two consecutive cycle_indices, sub-pos differs (L24). |
| `ore_no_color_noise` | `peak_with_noise(seed, &ORE)` always equals ORE.peak_rgb regardless of seed (L8). |

Visual verification (deferred / manual):
- Run on a stock night-water map; eyeball for grid patterns or beat-sync.
- If suspicious, do option D — capture gamemd and our engine on the same scene, frame-diff.

Performance check (deferred, optional):
- Bench `build_sparkle_instances` with a synthetic 200-cell viewport; assert sub-100µs per call. Defer unless perf flagged.

## Architectural Decisions

**Patterns followed:**
- Render module producing `Vec<SpriteInstance>` consumed by `InstanceBufferPool` and dispatched in `draw_passes.rs` — identical convention to smudge, overlay, unit passes.
- Pure builder function — no GPU types in the build path, only in the dispatch step. Matches the build/dispatch split that the architecture enforces.
- Read-only borrows of sim state — sim → render dependency direction only, never the reverse.
- Constants in the module file (no INI), matching how other render-time constants (e.g., bit-font baselines) are co-located with the code that uses them.

**Patterns deviated from:**
- None.

**New patterns introduced:**
- `extra_animations: bool` field on `GraphicsConfig`. New but minimal — a single boolean for a player-facing toggle. Future-proof for the other gamemd "Extra Animations" gated effects (lasers, particle systems, line trails).

**Tech debt introduced:**
- The 2500ms cycle bucket is a stateless approximation of gamemd's variable per-cycle sequencing. Visible only in frame-by-frame diff analysis. Documented in this design; revisit if option-D pixel-diff audit flags the drift.

## Alternatives Considered

**Approach B from the brainstorm: Simplified — fixed cycle length per species.**
- Same module structure, but `LerpSpeed` is fixed per species instead of hashed per cell-cycle.
- Rejected: produces synchronized-group pulsing (cells with the same offset all peak together over time), which is a visible artifact in normal play. Fails the parity bar.

**Stateful CellSparkle struct (matches gamemd 1:1).**
- Each cell carries a persistent `CellSparkle` ticked per frame.
- Rejected during brainstorm Step 3 in favor of stateless: lifecycle management, allocation churn, and per-cell mutable state in the render layer added complexity for no observable benefit (the stateless approximation matches gamemd within the parity bar).

**Wall-clock time source.**
- gamemd's `timeGetTime` ticks during pause.
- Rejected: violates replay determinism (replays would render different sparkle frames depending on wall time). Cosmetic-only divergence on pause is acceptable.

**Continuous cycling (no timer pause).**
- Drop gamemd's TimerInit feature entirely; cells cycle dim→bright continuously.
- Rejected: cells would sparkle ~3× more frequently than gamemd, losing the "scattered random points of light" feel.

**Dedicated 1×1 white atlas / new pipeline.**
- Add a PixelFxAtlas with its own BatchTexture.
- Rejected in favor of reusing `SelectionOverlay::white_texture()` — same texel, zero new GPU resources.

## Known Parity Gaps (NOT in this design's scope — deferred follow-ups)

- **OreTwinkle (TWNK1) AnimClass spawn at scenario load** — independent system per report §14.3. Needs its own brainstorm (AnimClass-equivalent integration).
- **Other "Extra Animations" gated effects** — laser-beam pulses, particle systems, line trails. Each is a separate render system. The `extra_animations` config field is plumbed so they slot in later under the same toggle.
- **RGB565 quantization for exact gamemd-pixel match** — accepted drift; player-imperceptible per the parity bar.
- **Wall-clock-on-pause** — accepted drift; replay determinism wins.
- **`g_PrimarySurface vtable+0x70 == 2` mode check** — not applicable in wgpu (gamemd-specific 16-bit surface check).
