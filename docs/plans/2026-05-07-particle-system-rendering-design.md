# Particle System Rendering Pipeline — Design

## Goal

Wire `Simulation.particle_systems` into the render pipeline so that combat smoke,
refinery dump bursts, gas clouds, fire trails, and gap-generator clouds become
visible — at parity with `gamemd.exe`'s Layer 3 SHP rendering — and add the
per-tick state-AI advance the renderer depends on for animation, translucency,
and DeleteOnStateLimit termination.

Tier 3 (Spark, Railgun pixel rendering) stays deferred at the dispatch boundary;
Tier 2 BehavesLike (Smoke, Gas, Fire) is the live target.

## Architecture Context

**Sim side ([sim/particles/](src/sim/particles/)).** `ParticleSystemStore` is a
BTreeMap-backed store hung off `Simulation`. `tick_particle_systems` advances
every system once per tick (between turrets+combat and ore-growth phases);
spawn entry points are wired for combat, refinery dock, gap generators, and
area damage. `Particle` carries `coords: IVec3` (leptons), `type_id`,
`animation_state: u8`, `translucency: u8`, `previous_coords`, `marked_for_deletion`,
plus drift fields. `ParticleType.image: Option<String>` holds the SHP filename
("WCCLOUD1", "LGRYSMK1", "gaslrgmk", etc.). Per-tick AI lives in
[smoke.rs](src/sim/particles/smoke.rs), [gas.rs](src/sim/particles/gas.rs),
[fire.rs](src/sim/particles/fire.rs); Spark/Railgun spawns are filtered at
[spawn.rs:42](src/sim/particles/spawn.rs#L42) (warn + return None).

**Critical sim gap.** None of the three live tick functions advance
`particle.animation_state` today — it is set at spawn (`pt.start_state_ai`,
typically 0) and never moved. Per
[PARTICLESYSTEMCLASS_GHIDRA_REPORT.md §9.12.3](docs/research/PARTICLESYSTEMCLASS_GHIDRA_REPORT.md),
`ParticleClass::GetImageFrame` (FUN_0062d830) returns `animation_state` directly
for Smoke/Gas, so without the advance the renderer would pin every particle to
frame 0 for its full lifetime, and `Translucent25State` / `Translucent50State` /
`DeleteOnStateLimit` would never trigger. The state-AI advance must land in the
same change set as the renderer.

**Render side ([app_render/](src/app_render/), [app_instances/](src/app_instances/)).**
Per-frame instance builders fill `WorldInstances` phase struct in
[build_instances.rs](src/app_render/build_instances.rs), which is then uploaded
to the GPU buffer pool and dispatched via 10 numbered draw steps in
[draw_passes.rs](src/app_render/draw_passes.rs):

```
1. Terrain (zdepth)
2. Bridge body (zdepth)
3. Overlays (passthrough)        3.5. Smudges
4. Bridge entities (Y-merge)
5. Ground objects + walls (Y-merge, Layer 2)
6. Building turrets
7. Cliff redraw (zdepth + Less)
8. Debug overlays
9. Shroud multiply
10. UI / sidebar
```

World-position effects, damage fires, garrison muzzle flashes, and parachute
anims all funnel into `shp_paged` and Y-sort with Layer 2 entities via
`merge_passes::draw_merged_object_pass`. Particles are different: they want
gamemd Layer 3 — above all Layer 2 objects, above turrets, above cliff redraw,
below shroud / debug / UI.

**SHP atlas registration ([sprite_atlas.rs:653-723](src/render/sprite_atlas.rs#L653)).**
At map-load, world-effect SHPs are pre-rendered into the multi-page sprite
atlas using `anim.pal` (effect palette, not unit.pal). Names tracked in
`effect_type_ids`. Particle SHPs are not registered today — atlas lookups
silently miss.

**Coordinate space mismatch.** Particles work in lepton coordinates (256
leptons = 1 cell). The existing `terrain::iso_to_screen(rx, ry, z)` takes
integer cell coords + height byte. There is no lepton-to-screen helper. The
particle renderer needs sub-cell precision, so a new helper is required.

## Impact Analysis

**Files added:**
- `src/app_instances/particles.rs` — new instance builder.

**Files modified:**

| File | Change |
|------|--------|
| [src/sim/particles/system_ai.rs](src/sim/particles/system_ai.rs) | New `advance_state` helper called from each Tier-2 tick; sub-tick counter on Particle. |
| [src/sim/particles/mod.rs](src/sim/particles/mod.rs) | One new `Particle` field: `state_advance_counter: u8` (sub-tick accumulator). |
| [src/sim/particles/smoke.rs](src/sim/particles/smoke.rs) | Call `advance_state` inside `tick_particle`. |
| [src/sim/particles/gas.rs](src/sim/particles/gas.rs) | Call `advance_state` inside `tick_particle`. |
| [src/sim/particles/fire.rs](src/sim/particles/fire.rs) | Call `advance_state` inside `tick_particle`. |
| [src/render/sprite_atlas.rs](src/render/sprite_atlas.rs) | New step 1f: register all `ParticleType.image` SHPs into `effect_type_ids` + `needed`. |
| [src/app_instances/mod.rs](src/app_instances/mod.rs) | Add `mod particles; pub(crate) use particles::*;`. |
| [src/app_render/build_instances.rs](src/app_render/build_instances.rs) | Add `particle_paged: Vec<Vec<SpriteInstance>>` to `WorldInstances`; call `build_particle_instances`; sort. |
| [src/app_render/mod.rs](src/app_render/mod.rs) | Upload `particle_p0..p3` keys to pool. |
| [src/app_render/draw_passes.rs](src/app_render/draw_passes.rs) | New Step 7.5 between cliff redraw and debug; pass `particle_paged` through `DrawPassData`. |
| [src/map/terrain.rs](src/map/terrain.rs) | Add `lepton_to_screen(coords: IVec3) -> (f32, f32)` helper. |
| [src/app_building_anim.rs:341](src/app_building_anim.rs#L341) `consume_bale_events` | Coordinate fix: `rx*256` → `rx*256 + 128`, `ry*256` → `ry*256 + 128`. |

**Files unchanged (deliberate):**
- `sim/world/world_hash.rs` already hashes `particle_systems`. The new
  `state_advance_counter` field is added to `Particle`, which means
  hashing must include it. State-hash extension is a one-line add to the
  same file.
- `sim/particles/wind.rs`, `system_ai::tick_one_system` dispatch — unchanged,
  state-AI advance is dispatched per-particle, not per-system.

**Risk areas:**
- **Determinism.** State-AI advance writes `animation_state` and `translucency`
  fields that already enter the world hash via the existing per-particle hash
  path. Adding `state_advance_counter` to the hash + ensuring it's also
  serialized in any future snapshot is required. Without it, two replays could
  diverge on the next-tick frame-index decision.
- **Atlas size growth.** Worst case: ~6 SHP filenames (LGRYSMK1, SGRYSMK1,
  WCCLOUD1, gaslrgmk, TXGASG, TXGASR), each ~20 frames, ~64×64 px = ~5 MB
  uncompressed across all frames. Within current multi-page atlas headroom.
  None of the particle SHPs have facings (single direction).
- **Draw-call cost.** A worst-case combat scene might have ~50 active particle
  systems × ~15 particles each = 750 SpriteInstances. Sorted within one pass,
  one draw call per atlas page. Cheap.
- **Layer 3 ordering vs cliff.** Particles drawn AFTER cliff redraw means a
  cliff face that's screen-Y-closer than a particle still gets occluded by
  the particle. That's the gamemd behavior (Layer 3 > everything). If the
  player ever notices a smoke plume floating in front of a cliff face that
  should occlude it, that's a tunable bias question, not an architectural one.
- **Lepton-to-screen helper.** Sub-cell precision math must round consistently
  with `iso_to_screen` to avoid 1-pixel jitter when a particle crosses a cell
  boundary. Tested with explicit integer-leptons-to-screen unit tests.

## Chosen Approach

A single instance builder dispatches on `behaves_like` for the frame-index
calculation and uploads to a dedicated `particle_paged` GPU buffer set, drawn
in a new Step 7.5 between cliff redraw and debug overlays. SHP atlas
registration extends the existing `effect_type_ids` channel. The state-AI
advance is added in-place inside the Tier-2 tick functions, gated by
`StateAIAdvance` per gamemd's formula, with translucency-state transitions
mapped to the byte values the renderer reads.

Tier 3 (Spark, Railgun) is stubbed with a defensive once-per-type warn-log at
the render dispatch — defense in depth against a future spawn-side bug or a
future-build snapshot leaking a Tier-3 system into the store.

The corner→center spawn-coord fix at [app_building_anim.rs:341](src/app_building_anim.rs#L341)
rides along, since it's a one-line change in the consumer and now becomes
testable end-to-end.

## Tiny-Detail Ledger

Every parity-relevant detail the implementation must preserve. Each item cites
its source.

### State-AI machine (sim side)

| # | Detail | Source |
|---|--------|--------|
| L1 | State advance denominator: `(num_loop_frames % 2 + 1) + StateAIAdvance`. (Note: `num_loop_frames` is the SHP's frame count, not the INI key.) | [doc §9.12.3, §3.6] |
| L2 | Sub-tick counter increments by 1 each tick; advance triggers when `counter % denominator == 0`. | [doc §3.6] |
| L3 | When `animation_state == EndStateAI`: if `DeleteOnStateLimit`, mark for deletion; otherwise reset `animation_state = 0`. | [doc §3.8 Smoke; §9.12.3] |
| L4 | When `animation_state >= Translucent50State` (and key not 0xFF), set `translucency = 0x19`. | [doc §3.8 Fire; §9.7] |
| L5 | When `animation_state >= Translucent25State` (and key not 0xFF), set `translucency = 0x32`. | [doc §3.8 Fire; §9.7] |
| L6 | Default `Translucent25State = 0xFF` and `Translucent50State = 0xFF` mean "never". Already encoded in [particle_type.rs:217-222](src/rules/particle_type.rs#L217). | [doc §4.2; doc §9.2] |
| L7 | `final_damage_state` defaults to `EndStateAI` value when key absent. Already correct in [particle_type.rs:170](src/rules/particle_type.rs#L170). | [doc §9.2] |
| L8 | State-AI advance runs **before** lifetime decrement, so a state hitting EndStateAI in the same tick as lifetime hitting 0 still respects DeleteOnStateLimit semantics. | [doc §3.2 ordering] |

### Frame index (render side)

| # | Detail | Source |
|---|--------|--------|
| L9 | Smoke (BehavesLike=1 particle-side): `frame = animation_state` directly. | [doc §9.12.3] |
| L10 | Gas (BehavesLike=0 particle-side): `frame = animation_state` directly. | [doc §9.12.3] |
| L11 | Fire (BehavesLike=2 particle-side): `frame = facing_band * pt.end_state_ai + animation_state` where `facing_band = particle.facing` mod 4. (Tier-2 fire spawning uses fixed facing per system; this still applies.) | [doc §9.12.3] |
| L12 | Spark/Railgun (3, 4): renderer dispatch arm logs `warn!` once per type and skips. | spawn-side parity at [spawn.rs:42](src/sim/particles/spawn.rs#L42) |

### Translucency byte → alpha (render side)

| # | Detail | Source |
|---|--------|--------|
| L13 | `translucency == 0x00` → alpha `1.0` (opaque). | [GHIDRA 0x0062cec0] |
| L14 | `translucency == 0x19` (25 dec) → alpha `0.5` (50% translucent → flag 0x2802). | [doc §8.7, §9.7] |
| L15 | `translucency == 0x32` (50 dec) → alpha `0.25` (25% translucent → flag 0x2804). | [doc §8.7, §9.7] |
| L16 | `translucency >= 0x4A` (74 dec) → alpha `~0.16` (very faded → flag 0x2806). | [doc §8.7] |
| L17 | Translucency-flag application is gated on game speed (`DAT_00a8eb78 == 2`) in gamemd. We always run "normal speed" — apply the mapping unconditionally. | [doc §9.7] |

### SHP atlas + palette

| # | Detail | Source |
|---|--------|--------|
| L18 | Particle SHPs use `anim.pal` (effect palette), not unit.pal. Loaded via the `Image=` field through `ObjectTypeClass::ReadINI`. | [doc §11.4] |
| L19 | Atlas miss → silent skip per particle (no fallback animation, no error). gamemd does `if (shp_surface == NULL) return`. | [doc §11.4, §8.7] |
| L20 | `house_color` slot is forced to `HouseColorIndex(0)` for particles — no owner tint. | gamemd CC_Draw_Shape remap=0 |
| L21 | `facing` slot is forced to `0` in the atlas key (single-direction SHPs; PARACH-style registration). | particle SHPs have no `Facings=` |
| L22 | Frame range registered: `0..frame_count` where `frame_count` comes from `shp.frames.len()` (not `len()/2`, since particle SHPs have no shadow frames). Mirrors `effect_names` path. | [sprite_atlas.rs:701-707](src/render/sprite_atlas.rs#L701) — particle SHPs use full frame count |

### Draw layer + ordering

| # | Detail | Source |
|---|--------|--------|
| L23 | `ParticleClass::GetLayer = 3` for all particles. Drawn above Layer 2 (buildings/units/turrets) and above cliff redraw (Step 7), below debug/shroud/UI. | [GHIDRA 0x0062d770; doc §5.3] |
| L24 | Within the particle pass, sort by depth descending (back-to-front) so translucency blending stacks correctly. | standard alpha-blending requirement |
| L25 | Z-adjust = `-15 - AdjustForZ()` in gamemd lifts particles 15 px above the iso ground plane. In our depth-buffer-free Layer-3 pass, this is irrelevant for occlusion (we're above everything) but applies as a screen-Y nudge so the smoke origin sits just above the spawn coord, not at it. | [doc §9.7] |
| L26 | Centered draw flag (0x2000): the SHP frame is drawn centered on the particle screen position. Atlas entries already include `offset_x/offset_y` for sprite-center anchoring. | [doc §8.7] |
| L27 | Frame skip on fast-forward (Smoke=1, Spark=3): not implemented — engine has no fast-forward mode equivalent. Documented as known absent. | [doc §8.7] — not applicable |
| L28 | Fog-of-war check (SpecialFlags & 0x1000): TS-legacy, FogOfWar=false in YR — explicitly NOT implemented. | [doc §9.10, CLAUDE.md TS-Ghosts] |

### Coordinate transform

| # | Detail | Source |
|---|--------|--------|
| L29 | Particle world coords are in leptons; 256 leptons = 1 cell. Sub-cell offset in screen space: `dx = (sub_x - sub_y) * TILE_WIDTH/2 / 256`, `dy = (sub_x + sub_y) * TILE_HEIGHT/2 / 256`. Z height in leptons → screen Y offset using same `HEIGHT_STEP` as `iso_to_screen`. | [Particle.coords: IVec3 leptons; iso math from terrain.rs:187-191](src/map/terrain.rs#L187) |
| L30 | Lepton division uses Euclidean rules (negative leptons go to lower cell, not toward zero). Particles can drift to negative coords near the map edge; rounding-toward-zero would put them on the wrong cell. | numerical correctness |

### View culling

| # | Detail | Source |
|---|--------|--------|
| L31 | Off-screen particles are culled from the instance build. Use the same in-view check pattern as `build_world_effect_instances` ([overlays.rs:76](src/app_instances/overlays.rs#L76)) with a 120 px margin. | matches existing world-effect culling |
| L32 | Visibility: particles are not gated on fog/shroud at the instance build level. The shroud multiply pass (Step 9) handles per-pixel occlusion of shrouded cells consistently with terrain/entities. | matches existing entity/world-effect behavior |

### Spawn-coord fix (rider task)

| # | Detail | Source |
|---|--------|--------|
| L33 | `consume_bale_events` at [app_building_anim.rs:341](src/app_building_anim.rs#L341): change `rx as i32 * 256` → `rx as i32 * 256 + 128` and same for `ry`. gamemd's `BuildingClass::GetCoords` returns top-left cell **center**. | [doc §11.8.C; harvester dock trace] |

## Design

### Component layout

```
sim/particles/system_ai.rs
  └─ pub(super) fn advance_state(p: &mut Particle, pt: &ParticleType, image_frame_count: u16)
       Called from smoke::tick_particle, gas::tick_particle, fire::tick_particle.
       Owns the formula in L1/L2/L3/L4/L5/L8.

sim/particles/mod.rs
  └─ Particle.state_advance_counter: u8        (new field)

app_instances/particles.rs                     (NEW)
  └─ pub(crate) fn build_particle_instances(state: &AppState,
                                            paged: &mut [Vec<SpriteInstance>])
       Iterates Simulation.particle_systems, dispatches frame index on
       behaves_like, looks up atlas entry, emits SpriteInstance per particle.
       Defensive Spark/Railgun arm with once-per-type log.

app_render/build_instances.rs
  └─ WorldInstances.particle_paged: Vec<Vec<SpriteInstance>>     (new field)
  └─ build_world_instances() now allocates particle_paged, calls
       build_particle_instances, sorts each page descending by depth.

app_render/mod.rs
  └─ upload_to_gpu() uploads "particle_p0".."particle_p3" pool keys.

app_render/draw_passes.rs
  └─ DrawPassData.particle_paged: &'a [Vec<SpriteInstance>]      (new field)
  └─ Step 7.5: between cliff redraw and debug, draws each particle page
       through the existing passthrough pipeline (no depth read/write).

map/terrain.rs
  └─ pub fn lepton_to_screen(coords: IVec3) -> (f32, f32)        (new helper)
       Subdivides leptons into cell + sub-cell offset, applies iso math.
```

### State-AI advance (sim side)

In `sim/particles/system_ai.rs`:

```rust
/// Advance one particle's animation state machine by one tick. Implements the
/// formula at gamemd FUN_0062f9a0 / FUN_0062ed40 (per BehavesLike).
///
/// Caller passes `image_frame_count` because we don't pull SHP frame counts
/// into sim — render layer cached the count at atlas-load and threads it back
/// here through the rules registry. (See "Frame count plumbing" below.)
pub(super) fn advance_state(p: &mut Particle, pt: &ParticleType, image_frame_count: u16) {
    let denom = (image_frame_count % 2 + 1) as u8 + pt.state_ai_advance;
    p.state_advance_counter = p.state_advance_counter.wrapping_add(1);
    if p.state_advance_counter % denom != 0 {
        return;
    }
    p.animation_state = p.animation_state.saturating_add(1);
    if p.animation_state == pt.end_state_ai {
        if pt.delete_on_state_limit {
            p.marked_for_deletion = true;
        } else {
            p.animation_state = 0;
        }
    }
    if pt.translucent_50_state != 0xFF && p.animation_state >= pt.translucent_50_state {
        p.translucency = 0x19;
    }
    if pt.translucent_25_state != 0xFF && p.animation_state >= pt.translucent_25_state {
        p.translucency = 0x32;
    }
}
```

Called from `smoke::tick_particle`, `gas::tick_particle`, `fire::tick_particle`
**before** the existing `lifetime_remaining` decrement. This preserves the
gamemd ordering where state-AI runs before lifetime check (L8).

**Frame count plumbing.** The denominator needs the SHP's frame count.
`active_anim_frame_counts: HashMap<String, u16>` already exists on the sprite
atlas at [sprite_atlas.rs:715-717](src/render/sprite_atlas.rs#L715) for
exactly this purpose (chrono-teleport). Particle frame counts get added the
same way. The sim accesses them through the existing rules registry by adding
an `image_frame_count: Option<u16>` cached on `ParticleType` after atlas-load
resolution. (Alternative: sim takes a `&FrameCountResolver` parameter through
`tick_particle_systems`; rejected because every other rules-derived constant
already lives on the type registry.)

### Per-particle SpriteInstance build

`app_instances/particles.rs`:

```rust
pub(crate) fn build_particle_instances(state: &AppState, paged: &mut [Vec<SpriteInstance>]) {
    let (sim, atlas, rules) = match (&state.simulation, &state.sprite_atlas, &state.rules) {
        (Some(s), Some(a), Some(r)) => (s, a, r),
        _ => return,
    };

    let z = state.zoom_level;
    let (cam_x, cam_y, sw, sh) = (
        state.camera_x, state.camera_y,
        state.render_width() as f32 / z,
        state.render_height() as f32 / z,
    );

    for (_sys_id, sys) in sim.particle_systems.iter() {
        let pst = rules.particle_system_type(sys.type_id);
        match pst.behaves_like {
            ParticleSystemBehavesLike::Spark | ParticleSystemBehavesLike::Railgun => {
                warn_once_per_tier3_type(pst.behaves_like);
                continue;
            }
            _ => {}
        }
        for p in &sys.particles {
            let pt = rules.particle_type(p.type_id);
            let Some(image_name) = pt.image.as_deref() else { continue };

            // Frame index per L9/L10/L11.
            let frame = match pt.behaves_like {
                ParticleBehavesLike::Smoke | ParticleBehavesLike::Gas => p.animation_state as u16,
                ParticleBehavesLike::Fire => {
                    let facing_band = (sys.facing as u16 / 0x40) & 0x3;
                    facing_band * pt.end_state_ai as u16 + p.animation_state as u16
                }
                _ => continue,
            };

            // Atlas lookup (silent miss per L19).
            let key = ShpSpriteKey {
                type_id: image_name.to_string(),
                facing: 0,
                frame,
                house_color: HouseColorIndex(0),
            };
            let Some(entry) = atlas.get(&key) else { continue };

            let (sx, sy) = terrain::lepton_to_screen(p.coords);
            let sy_lifted = sy - 15.0;   // L25: gamemd's -15 z-adjust as a screen-Y nudge.

            if !in_view(sx, sy_lifted, 64.0, 64.0, cam_x, cam_y, sw, sh, 120.0) {
                continue;
            }

            // Translucency byte → alpha (L13/L14/L15/L16).
            let alpha = match p.translucency {
                0x00 => 1.0,
                0x19 => 0.5,
                0x32 => 0.25,
                t if t >= 0x4A => 0.16,
                _ => 1.0,
            };

            // Depth: lepton-y for back-to-front Y-sort within the pass.
            // No depth read/write at draw time — depth field used only for
            // CPU-side sort ordering.
            let depth = (sy_lifted * 100.0) as f32 / 1_000_000.0;  // dummy bucketing

            paged[entry.page as usize].push(SpriteInstance {
                position: [sx + entry.offset_x, sy_lifted + entry.offset_y],
                size: entry.pixel_size,
                uv_origin: entry.uv_origin,
                uv_size: entry.uv_size,
                depth,
                tint: [1.0, 1.0, 1.0],   // L20: no owner tint
                alpha,
            });
        }
    }
}
```

(The exact depth-encoding line above is illustrative — the implementation
plan will pin it to the Y-sort scheme used by the surrounding code.)

### SHP atlas registration

In [sprite_atlas.rs](src/render/sprite_atlas.rs) — extend the post-1d block
that builds `effect_names`:

```rust
// Step 1f: Pre-load all ParticleType.Image SHPs.
// Per gamemd, particles route through ObjectTypeClass::ReadINI → Image= →
// LoadFileFromMIX. They use anim.pal (effect palette).
if let Some(r) = rules {
    for pt in r.particle_types_iter() {
        if let Some(image) = pt.image.as_deref() {
            if !effect_names.iter().any(|n| n.eq_ignore_ascii_case(image)) {
                effect_names.push(image.to_string());
            }
        }
    }
}
```

The existing loop at [sprite_atlas.rs:696-722](src/render/sprite_atlas.rs#L696)
then handles MIX lookup, frame-count detection, atlas registration, and
effect-palette flagging without further change. Frame counts also seed
`active_anim_frame_counts` so the sim's state-AI advance can resolve them.

### Draw step

In [draw_passes.rs](src/app_render/draw_passes.rs), insert between Step 7
(cliff redraw) and Step 8 (debug):

```rust
// --- Step 7.5: Particles (Layer 3, above all ground geometry incl. cliffs) ---
// gamemd ParticleClass::GetLayer returns 3 for all particles, drawing them
// above Layer 2 (buildings, units, turrets) and Z-buffered to be above
// cliffs. We achieve the same by drawing AFTER cliff redraw with the
// passthrough pipeline (no depth interaction). Each atlas page is a
// separate pool key.
const PARTICLE_KEYS: [&str; 4] = [
    "particle_p0", "particle_p1", "particle_p2", "particle_p3",
];
for (i, key) in PARTICLE_KEYS.iter().enumerate() {
    if let Some(page) = state.sprite_atlas.as_ref().and_then(|a| a.page(i)) {
        if let Some((buf, count)) = pool.get(key) {
            state.batch_renderer.draw_passthrough_range(
                &mut pass, &page.texture, buf, 0, count,
            );
        }
    }
}
```

Reuses the existing passthrough pipeline (the same one that draws
`overlay_bridge_detail` and smudges) so no new shader / pipeline state.

### Lepton-to-screen helper

In [terrain.rs](src/map/terrain.rs):

```rust
/// Convert lepton-world coords (256 leptons = 1 cell) to screen pixels.
///
/// Used by the particle renderer and any future system that needs sub-cell
/// precision. Z is in leptons; HEIGHT_STEP_LEPTONS is the per-Z-level lift.
pub fn lepton_to_screen(coords: IVec3) -> (f32, f32) {
    const LEPTONS_PER_CELL: i32 = 256;
    let cell_x = coords.x.div_euclid(LEPTONS_PER_CELL);
    let cell_y = coords.y.div_euclid(LEPTONS_PER_CELL);
    let sub_x = coords.x.rem_euclid(LEPTONS_PER_CELL) as f32;
    let sub_y = coords.y.rem_euclid(LEPTONS_PER_CELL) as f32;

    let cell_center_sx = (cell_x as f32 - cell_y as f32) * TILE_WIDTH  / 2.0;
    let cell_center_sy = (cell_x as f32 + cell_y as f32) * TILE_HEIGHT / 2.0
                       + TILE_HEIGHT / 2.0;

    let dx = (sub_x - sub_y) * (TILE_WIDTH  / 2.0) / LEPTONS_PER_CELL as f32;
    let dy = (sub_x + sub_y) * (TILE_HEIGHT / 2.0) / LEPTONS_PER_CELL as f32;

    let z_lift = coords.z as f32 / LEPTONS_PER_CELL as f32 * HEIGHT_STEP;

    (cell_center_sx + dx, cell_center_sy + dy - z_lift)
}
```

Tested with: zero coord, integer cell coord, sub-cell offset, negative coord
(map edge), Z lift.

### Spawn-coord fix

[app_building_anim.rs:341](src/app_building_anim.rs#L341) `consume_bale_events`:

```diff
- let coords = IVec3::new(rx as i32 * 256, ry as i32 * 256, ...);
+ let coords = IVec3::new(rx as i32 * 256 + 128, ry as i32 * 256 + 128, ...);
```

Once the renderer exists this becomes a visual test: refinery dump smoke
should bloom from the cell center, not from the cell's NW corner.

### Determinism / replay

The state-AI advance writes only fields that are already part of the world
hash via the existing per-particle hash path (`animation_state`, `translucency`,
`marked_for_deletion`). The new `state_advance_counter` field must also be
hashed — added to the same per-particle hash function in
`sim/world/world_hash.rs`. Without it, two replays diverging only in
state-counter parity would silently desynchronize when `delete_on_state_limit`
fires on a different tick.

The state-AI advance runs **inside** `tick_particle_systems`, which is
already in the deterministic phase ordering enforced by `World::advance_tick`.
No new tick-phase sequencing is introduced.

`Particle::state_advance_counter` is a `u8` initialized to 0 at spawn. The
`saturating_add(1)` becomes `wrapping_add(1)` (a `u8` rolls over every 256
ticks; rollover does not affect the modulo check because `denom <= 8` in
practice — `(num_loop_frames % 2 + 1) + StateAIAdvance`, and `StateAIAdvance`
defaults to 4). Wrapping is intentional and replay-safe.

Snapshot serde (per memory `project_snapshot_serialization.md`): the new
`state_advance_counter` field is added to `Particle`, which already derives
the relevant `Serialize`/`Deserialize` traits at that future change point.
No schema migration concerns — particle snapshots don't exist yet, so no
compatibility burden.

### Error handling

| Failure mode | Handling |
|--------------|----------|
| `ParticleType.image` is `None` (Tier-3 types like Spark) | Silent `continue` per particle. Already enforced by Tier-3 system filter at the dispatch. |
| Atlas miss (SHP failed to load from MIX) | Silent `continue` per particle, matching gamemd L19. Atlas-load logs the warn at startup; per-frame silence is intentional. |
| Frame index out of range (`frame >= shp.frames.len()`) | Atlas lookup returns None → silent continue. The state-AI machine never advances past `EndStateAI` (resets or deletes), so this is in practice unreachable for correctly-defined types. |
| `ParticleSystem` with Tier-3 BehavesLike sneaks into store | Defensive arm logs `warn!` once per type via static `OnceLock<HashSet>`, then `continue`. |
| Lepton coord overflow (i32) | `div_euclid`/`rem_euclid` correctly handle negatives; bounded by map size, no overflow possible at IVec3 ranges. |

### Testing strategy

**Unit tests:**
- `lepton_to_screen` — origin, integer cell, sub-cell offsets in all four
  isometric quadrants, negative coords, Z-lift.
- `advance_state` — counter rollover, EndStateAI with DeleteOnStateLimit,
  EndStateAI without DeleteOnStateLimit (resets), Translucent50State /
  Translucent25State transitions, denominator math for SHP frame counts
  parity ↔ odd.
- `build_particle_instances` — empty store yields no instances; one Smoke
  particle yields one instance on the correct page; Tier-3 system yields
  no instances + warn-log fired once; out-of-view particles culled.

**Integration test:** spawn one BigGreySmokeSys system, advance 50 ticks,
assert particle count > 0 and that `build_particle_instances` produces the
expected number of SpriteInstances on a known atlas page.

**Determinism test:** add to existing `world_hash` test pattern — two
sim runs from the same seed with the same inputs produce identical hashes,
including the new `state_advance_counter` field on each particle.

**Visual end-to-end (no automated check):**
1. Skirmish, build a refinery, miner dumps ore — refinery dump smoke should
   bloom from cell center, animate through the SHP frames, and fade out at
   Translucent50State / Translucent25State.
2. Damage a building below ConditionYellow — smoke plume should rise above
   the building roof (Layer 3, above turrets).
3. Spam a Yuri Disk weapon (psychic gas) — gas cloud should drift, fade,
   and dissipate via NextParticle chain.

## Architectural Decisions

**Patterns followed:**
- Per-content-class instance builder in `app_instances/`, mirroring `units.rs` /
  `shp.rs` / `overlays.rs`.
- SHP atlas registration via `effect_type_ids` channel (anim.pal palette),
  same as world effects, damage fires, garrison flashes.
- Y-sort within the pass, back-to-front, descending depth — matches every
  other sprite pass.
- Sim-side state lives in `sim/particles/`; render reads through `&AppState`
  with no sim/render coupling — `sim/` invariant from CLAUDE.md preserved.
- New tick-time logic added inside existing tick functions, not in a new
  phase — preserves current `World::advance_tick` order.

**Patterns deviated from:**
- New draw step (Step 7.5) inserted between cliff and debug. World effects
  / damage fires / garrison flashes / parachute anims all live in the
  Layer 2 Y-merge today; particles deliberately don't, for parity with
  gamemd's Layer 3.
- Single-builder dispatch on `behaves_like` rather than one builder per
  variant. Justified by ~80% shared code; can split later if Tier 3
  pixel rendering forces divergence.

**Tech debt introduced:**
- The damage_fire / world_effect / parachute / garrison-flash builders
  remain Layer-2-Y-merged. If parity demands they also be Layer 3 (TBD —
  needs separate verification against gamemd `GetLayer` for AnimClass
  vs ParticleClass), they would migrate to the same Step 7.5. This is
  not a regression — it's existing drift this work doesn't address.
- Frame count threading from atlas-load → rules registry → sim
  introduces a one-way dependency at load time. Acceptable; matches the
  existing chrono-teleport frame-count flow.

## Alternatives Considered

**Layer 2 Y-merge fold (Q2c).** Reuses pipeline as-is, zero new draw step.
Rejected: refinery smoke would render behind the refinery roof when its
screen-Y is lower than the roof's anchor — a visible parity drift in
exactly the use case that triggered this work.

**One builder per BehavesLike (Q3b).** Cleaner if Tier 3 lands with very
different pixel-render code. Rejected: the variants today differ in 2 lines
of frame-index math; per-variant builders would be 90% boilerplate copy.
Trivial to split if Tier 3 forces it.

**Trait dispatch (Q3c).** Most flexible. Rejected: violates YAGNI for a
3-way branch with mostly shared code; adds vtable indirection in a hot
loop.

**Render frame 0 only, defer state machine (Q1b).** Fastest path to
"something visible." Rejected: ships a known parity drift (smoke that
doesn't dissipate) on the same change that's supposed to close the
"particles invisible" gap — net neutral on the parity ledger.

**Block renderer on a separate state-AI task (Q1c).** Rejected: same total
work either way; sequencing it in this task closes both gaps in one ship,
and the state machine has zero new architecture.

**Trust spawn-side filtering, no Tier-3 render arm (Q5a-i).** Rejected
in favor of defensive once-per-type log: ~3 lines, protects against a
future spawn-side bug or future-build snapshot leaking a Tier-3 system
into the store.

## Open Follow-ups (intentional, not blocking)

These are out-of-scope per session context but the design must not
preclude them:
- Multi-purifier credit stacking on bale deposit.
- RefinerySmokeFrames semantics (UNVERIFIED).
- Slot 7/8 trigger emission on dock arrival/empty (no-op for stock
  GAREFN/NAREFN; mod-incompatibility only).
- Tier 3 Spark/Railgun pixel rendering (separate task; renderer dispatch
  arm is already a stub).
- Entity-attached following for damage smoke on moving units (already a
  deferred follow-up in [smoke.rs](src/sim/particles/smoke.rs) module
  header).
- 25%-on-even-frame random drift (also deferred in smoke.rs).
- Bridge collision in `move_smoke` / `move_gas` (deferred).
- AnimClass vs ParticleClass `GetLayer` audit — verify whether damage
  fires / world effects / garrison flashes / parachute anims should also
  migrate to Layer 3 for full parity. Not blocked by this work; they
  remain in the existing Y-merge.
