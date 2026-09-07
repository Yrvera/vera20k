# Per-pixel sprite Z-buffer parity — implementation plan

Date: 2026-09-07. Branch: `feature/zbuffer-sprite-ztest-docs`.
Source revision: `c9d07488` (docs corrected, no code yet).

## Intended outcome

Player-visible: a unit standing behind a tall building is hidden pixel by pixel
behind the building's upper part and visible beside its base, as in gamemd.
Buildings lose pixels to nearer walls and cliffs. Units never occlude each
other through depth; their mutual order stays the layer Y-sort.

Native mechanism (established from leaf bodies, see
[ZBUFFER_DEPTH_SYSTEM.md](../research/ZBUFFER_DEPTH_SYSTEM.md) Overview and
sections 1, 3, 4, 7):

| Draw | Flags | Leaf | Per pixel |
|---|---|---|---|
| Building body (`BuildingClass_DrawBody @ 0x0043d290`) | `0x6E00` | `0x004990e0` (format 3) / `0x004958d0` (format 1) | `base_z - zshape[x] < zbuf[x]` → draw and **write** `base_z - zshape[x]` |
| Other objects (`TechnoClass__Draw @ 0x00706640`, VXL cache blit) | `0x2800` | `0x00494b60` / `0x00497fd0` | `base_z - zshape[x] < zbuf[x]` → draw, **no write** (zshape row is all zero) |
| TMP tiles, walls, overlays (`0x00547cf0`) | — | tile blitter | `base_z + zdata <= zbuf` → draw and write |

`base_z` is a 16-bit value in screen-row units, seeded per draw (section 1
"Base Z formula") and stepped per scanline by the gradient table
`0x00817710` (section 4). Lower Z is nearer. The building z-shape is
`BUILDNGZ.SHA` frame 0, each non-zero byte remapped by `-0x41` at load
(`FUN_0045e8f0`), placed at `(0xc6, 0x1be) + ZShapePointMove -
CellToPixel(foundation)` relative to the building draw point, and dropped
when the foundation width is `>= 8`.

## Current Rust (what changes)

| Concern | Today | Anchor |
|---|---|---|
| SHP object sprites | `overlay_passthrough_pipeline`: depth compare `Always`, no write | `src/render/batch.rs:702-703` |
| Voxel sprites | `voxel_sprite_pipeline`: `LessEqual`, no write, flat per-instance depth | `src/render/batch.rs:1007-1008` |
| Terrain / bridge bodies | `zdepth_pipeline`: `Less`, write, per-pixel `frag_depth = base - z_sample * scale` | `src/render/batch.rs:887-888`, `src/render/zdepth_shader.wgsl:80-102` |
| Per-pixel scale | terrain atlas `0.0002` per R8 unit; bridge bodies `255/world_height` | `zdepth_shader.wgsl:93-96` |
| Sprite depth | one flat value per instance: `1 - (iso_row - origin_y)/world_height - z*1e-4` | `src/app/presentation/instances/helpers.rs:362-371` |
| Draw-plan Z policy | `RenderZPolicy {None, ReadWrite, ReadOnly, AlphaReadWrite}` carried, not consumed | `src/render/tactical_draw_plan.rs:25-34` |
| Object pass | `draw_native_ground_object_pass` / `draw_merged_object_pass` bind passthrough for every SHP run | `src/app/presentation/render/merge_passes.rs:255,295,545,585` |
| Building body emission | `src/app/presentation/instances/shp.rs` (~300-400), bib at `emit_building_bib` (578) | |
| BUILDNGZ | not loaded; `ZShapePointMove` not parsed (`grep -ri zshapepointmove src/` empty). The earlier implementation is not in git history (`git log -S load_buildngz` finds only docs) | |
| Selected-building depth stamp | writes a colour-masked silhouette after shroud so the front bracket can be depth-tested | `src/app/presentation/render/draw_passes.rs:460-484` |

Ordering authority is unchanged: `tactical_draw_plan.rs` still decides
submission order. Depth only rejects fragments.

## Prerequisite results (Ghidra, read-only, 2026-09-07)

Recorded in `ZBUFFER_DEPTH_SYSTEM.md` section 4 "Per-class Z policy and
gradient". What the deltas consume:

| Class | Z policy | Gradient entry | Z-shape |
|---|---|---|---|
| Building body | read + write | 2 | BUILDNGZ (none when foundation width > 7) |
| Building bib, damaged extras | read + write | 0 | none |
| Infantry | read only | 2 | none |
| Vehicle SHP (no turret) | read only | locomotor `Z_Gradient`, default 2 | none |
| Vehicle SHP (turret) / vehicle VXL | composited off-screen without Z, then one read-only blit with `0x2800` | locomotor, default 2 | none |
| Aircraft | read only | locomotor | none |
| Building VXL turret | read only | (z-height `cell+0x10A`) | none |
| Anim (normal) | read only | 2 (0 when Flat) | none |
| Anim shadow, object shadow pass | shadow family | — | — |

Gradient table bytes confirmed. Foundation offset: `CellToPixel((W-1)*256,
(H-1)*256)` subtracted from `(0xC6, 0x1BE) + ZShapePointMove`; the
`ZShapePointMove` term is skipped for building types `0x12`/`0x13`
(unidentified). Retail YR `ZShapePointMove` is set on ten structures (GAWEAP,
NAWEAP, YAWEAP, GAREFN, NAREFN, YAREFN, GAAIRC, GADEPT, NADEPT, CASANF09);
everything else is 0,0. `BUILDNGZ.SHA` is in `ra2md.mix -> conqmd.mix`: one
uncompressed 396x477 frame, 169,488 non-zero pixels.

Still open, and they block only the numbers, not the pipelines:

1. **Resolved: the "z-height" arg is brightness.** `Rules+0x17D8` is
   `[AudioVisual] ExtraInfantryLight` (read at `0x0066B6E7`); DrawSHP a10 is
   the intensity term and `vtable+0x464` a brightness adjuster. VERA's lighting
   consumer is correct. The Z term is DrawSHP **a7** (z-adjust px, OpenTS
   `zadjust - 2` order): bib `-1 - AdjustForZ`, infantry global
   `[0x00825500]`, units locomotor `Z_Adjust` via `+0x2EC(gradient)`.
   **Still open:** the building body's a7 at `0x0043D85F`, and whether the
   section 1 base-Z formula's `z_height = CC param_10` is a7 mis-numbered.
   Re-read `SHP_StandardBlitter 0x004373B0` against the push sequence at
   `0x0070642B` before coding the base Z.
2. Whether an anim's `+0x190` can carry `0x4000` (write). Assume no.
3. Locomotor `Z_Gradient` overrides (`loco+0x3C`) for non-default locomotors
   (hover, fly, jumpjet). Default 2 covers ground vehicles.
4. `Extended_SHP_blitter` parameter semantics for the zeroed Z-shape positions
   (positional inference only).
5. One breakpoint on `0x004990e0` and `0x00494b60` in a retail run remains the
   cheap positive proof; do it during the capture session of scenario 1.

## Required deltas

### 1. One depth unit for everything: 1 native Z row = `1 / world_height`

Native tile Z-data, gradient steps and z-shape bytes are all screen-row units
against one 16-bit buffer. VERA's sprite depth already maps one world pixel
row to `1/world_height`; bridge bodies push `255/world_height` for the same
reason. The terrain atlas still uses the fixed `0.0002` per R8 unit, which
is not commensurable with sprite depth on most map sizes (a 3000-row world
gives `0.00033`).

Delta: terrain instances push `255/world_height` in `fx_params.w` like bridge
bodies, and the `select(0.0002, ...)` fallback in `zdepth_shader.wgsl:93` goes
away. Verify the tile Z-data sign convention (`pixel_z = base + zdata`, native
adds; the shader subtracts a positive sample to move nearer) against a cliff
capture before and after, since cliff occlusion of voxels is the only
production consumer today.

Design left open: whether terrain keeps `Less` or moves to `LessEqual` to
mirror the tile blitter's `<=` (sprites use `<`). Equal-depth ties between
tiles and sprites are the case to decide on; note the choice in the shader.

### 2. Building body: depth write shaped by BUILDNGZ

- **Asset.** Load `BUILDNGZ.SHA` frame 0 via the existing SHP reader (`.SHA`
  is SHP format). Apply `byte -= 0x41` to every non-zero byte at load and keep
  the result signed. Store as a single GPU texture whose sample yields a
  signed row offset (R8Snorm, or R8Unorm with a `+128` bias undone in the
  shader; pick one, document it next to the loader). Zero outside the image
  (clamp-to-border or explicit bounds test).
- **Art data.** Parse `ZShapePointMove=` (Point2D, default 0,0) into the
  building art record in `src/rules/art_data.rs`. Stock art.ini uses it on a
  handful of tall structures; check retail values before choosing a default.
- **Placement.** Per building instance, compute the z-shape origin in screen
  pixels: `draw_point + (0xc6, 0x1be) + ZShapePointMove -
  CellToPixel(foundation_w - 1, foundation_h - 1)` — the exact
  `CellToPixel(foundation)` form is in `BuildingClass_DrawBody`; read the
  body at `0x0043d85f` for the subtraction rather than copying OpenTS's
  `(Width*256 - 256, Height*256 - 256)`. Foundation width `>= 8` → no
  z-shape (gamemd value; OpenTS uses 6).
- **Base Z.** Gradient entry 2 (section 1, `field[5]=0` variant): `raw =
  DefaultZ + YOrigin - spriteHeight - screenY + 1 + z_adjust`, quantised to
  3-row steps, then +1 Z per 3 rows going down. In VERA depth: the top row of
  the quad is nearest, depth grows by `1/world_height` every third row. The
  `z_adjust` term is DrawSHP a7 (pixels), not the brightness arg a10; the
  building body's a7 value and the walker's exact use of it are the open items
  above. Map it onto the existing `compute_sprite_depth` inputs and keep the
  quantisation (integer steps are what the retail capture will show at the
  occlusion boundary).
- **Pipeline.** A building-body pipeline: depth compare `Less`, depth write
  on, fragment shader emits `frag_depth = row_depth(frag_y) -
  zshape(frag_xy) / world_height`. Either extend `zdepth_shader.wgsl` with a
  mode (fx_params) or add a sibling shader; the instance needs the z-shape
  origin and the gradient parameters. `SpriteInstance` has `fx_params`
  (`DrawState`, loc 9) — decide whether to widen `DrawState` or add a
  dedicated building-body instance layout. Widening changes every atlas
  path; a dedicated layout changes only the building emitter and one pass.
- **Emission.** In `shp.rs` the building body run must be lowered with
  `RenderZPolicy::ReadWrite` and carry the z-shape origin. The bib
  (`emit_building_bib`) is drawn inside `DrawBody` natively; confirm its flag
  word in the prerequisite pass before deciding whether it writes.

### 3. Every other object: per-pixel depth test, no write

- New `shp_zread_pipeline`: depth compare `Less`, no write, fragment depth
  from the per-row gradient (entry 0 or 1 per class, from the prerequisite
  table). `overlay_passthrough_pipeline` stays for draws that natively carry
  no Z (shadows, particles, UI, and anims if step-0 item 3 says so).
- `voxel_sprite_pipeline` moves from `LessEqual` to `Less` and gains the same
  per-row gradient. Native VXL cache blits go through `0x00497fd0`, which is
  the same read-only leaf with an all-zero z-shape row.
- Consume `RenderZPolicy` in `draw_plan_lowering.rs`: `ReadWrite` →
  building-body pipeline, `ReadOnly` → zread pipeline, `None` → passthrough.
  `AlphaReadWrite` maps to the same as `ReadWrite` until translucent building
  modes (cloak/warp, section 1 z-modes 1–4) get their own evidence; record
  that as a residual.
- `merge_passes.rs` binds the pipeline per run from the policy instead of
  calling `draw_passthrough_range` unconditionally (`:255, :295, :545, :585`).
  Page-run helpers (`unit_page_runs`, `draw_shp_atlas_page_runs`) keep their
  order guarantees; add the policy to the run key only if a run can mix
  policies.

### 4. Consumers to re-inspect

- **Selected-building depth stamp** (`draw_passes.rs:460-484`). Once building
  bodies write depth in the main pass, the stamp may be redundant or may fight
  the BUILDNGZ-shaped values (it stamps the flat silhouette). Decide: drop it
  and let the bracket test against real building depth, or keep it and stamp
  with the same shaped depth. Do not leave both.
- **Post-shroud bracket redraw** (`selection_brackets.rs:253`, drawn through
  `depth_test_pipeline`) depth-tests against whatever the stamp or the body
  wrote; re-check it once the stamp decision above is made. The shroud
  sample-back (`shroud_buffer.rs::sample_world`) reads the shroud buffer, not
  depth, and is unaffected.
- **`gsi_*` source-order tests** in `draw_passes.rs:890-1006` pin pass order;
  they should not move. If a new pass is inserted, update the anchors
  deliberately.
- **Walls.** Native walls write Z through the tile blitter; VERA draws walls
  through passthrough with no write (`draw_passes.rs:132`). A building beside
  a wall cannot lose pixels to it until walls write depth. Either include wall
  depth writes here (they are TMP-family, so the zdepth path fits) or record
  the residual with the acceptance scenario below marked partial.

### 5. Obsolete after this lands

- The `select(0.0002, ...)` fallback in `zdepth_shader.wgsl`.
- The "sprite passthrough is the native contract" sentences in
  `DRAWING_HELPERS_ENGINE_SUBSTRATE_SERVICE_STUDY.md` and the depth summary
  in `docs/system-map/` if it states it (run `python -m tools.system_map
  check` after editing).
- The `BUILDNGZ.SHA — Removed` subsection of `ZBUFFER_DEPTH_SYSTEM.md`
  section 10 becomes the implemented-status entry.

## Dependencies and order

1. Prerequisite Ghidra pass (read-only) → addendum commit.
2. Delta 1 (depth unit) → cliff capture before/after → commit.
3. Delta 2 asset + art parse + placement (no pipeline yet; unit-testable) →
   commit.
4. Delta 2 pipeline + emission, Delta 3 pipelines + lowering, Delta 4
   consumers → one coherent commit or PR; splitting them leaves buildings
   writing depth that nothing tests, which is harmless, or sprites testing
   depth that nothing writes, which is also harmless, so either order works
   but neither half is player-visible alone.
5. Docs/system-map updates.

## Acceptance scenarios

Each needs a retail reference. Retail captures come from `gamemd.exe -win`
through the oracle rig (`Documents\vera20k-oracle\tools\dxgi_capture`); VERA
frames from `src/render/screenshot.rs` / `frame_readback.rs`. Use the same
map, same cells, same facing, and compare the occlusion boundary pixels, not
the whole frame.

1. **Unit behind a tall building.** Setup: a pre-placed NACNST (or NATECH)
   and a Rhino tank on the cell directly north-east of the building's
   foundation so its hull overlaps the wall art; skirmish map with sandbox
   visibility. Action: render one frame in both engines. Expected: the tank
   pixels inside the building's z-shape near-region are absent in both; the
   boundary matches within one pixel row; the tank's pixels beside the base
   are present in both.
2. **Unit in front of the building.** Same setup, tank one cell south-west.
   Expected: tank fully visible in both (Y-sort draws it after; depth test
   passes because the tank's rows are nearer).
3. **Building beside a cliff.** Setup: NAPOWR placed on the low side of a
   cliff so its top overlaps the cliff face. Expected: cliff pixels win where
   the tile Z-data is nearer, matching the retail capture; no regression in
   voxel-behind-cliff (existing behaviour).
4. **Wall edge regression.** Setup: GAWALL ring around GAPILE. Expected: no
   halo, no missing wall pixels along the building edge (the artifact that
   motivated the earlier removal). If walls do not write depth yet, expected
   is "no worse than today" and the scenario is recorded partial.
5. **Two units overlapping.** Setup: two tanks on adjacent cells, one
   behind the other. Expected: pure Y-sort result, identical to today; the
   depth test must not hide any pixel of the rear unit that the front unit's
   sprite does not cover.
6. **Infantry** — same as scenario 1 with a Conscript, once the prerequisite
   pass fixes its policy.
7. **Regression suite.** `cargo test -p vera20k --lib render::` and
   `cargo test -p vera20k --lib app::presentation::render`; the
   `gsi_*` order pins and `unit_page_runs_preserve_equal_depth_layer_order`
   must still pass. One full `cargo test -p vera20k --lib` for the final
   candidate; report the literal `test result:` line.

## Residuals to record (not closed by this plan)

- Translucent z-modes 1–4 (cloak, warp) and their slot families
  (0x74/0x78/0x12c/0x130): not traced; buildings and units in those states
  keep whatever policy the lowering assigns by default.
- `Sqrt_Approx`-style numeric nits do not apply here, but the 16-bit wrap of
  the native Z-buffer (`DefaultZ = 0x8000`, dirty-rect clears) is not
  modelled; VERA clears the whole depth buffer per frame. No known
  player-visible effect.
- Shadows: still absent for objects; this plan does not add them. The shadow
  pass (`(flags & ~6) | 0x601`) is a separate mechanism.
