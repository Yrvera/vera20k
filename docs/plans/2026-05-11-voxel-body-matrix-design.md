# Voxel Body Matrix — Body Rocking + Slope-Tilt SLERP + Tilt-Aware Lighting Design

**Date:** 2026-05-11
**Scope:** Closes three voxel-system parity gaps identified in
[2026-05-11-disparity-scan-voxel.md](../gap-scans/2026-05-11-disparity-scan-voxel.md):
G1 (body rocking), G3 (slope-tilt SLERP transition), G4 (per-facing lighting LUT ignores
slope/rocking). Also closes the related G7 (slope-tilt translation shear offsets) as a
free pickup since it lives in the same body-matrix path.
**Source RE doc:** [BODY_ROCKING_GHIDRA_REPORT.md](../../../ra2-rust-game-docs/BODY_ROCKING_GHIDRA_REPORT.md)
**Status:** Design approved 2026-05-11. Ready for `/write-plan`.

## Goal

Make every voxel-bodied unit's body matrix vary per tick — driven by spring-damped
rocking angles, smoothly-interpolated slope transitions, and a lighting LUT that
follows the full body orientation — to match gamemd.exe's observable output on
vehicles in motion, vehicles taking weapon hits, vehicles crossing slope boundaries,
and EMP'd units.

## Architecture Context

The voxel render pipeline today is fundamentally **static**. At map-load,
[unit_atlas.rs](../../src/render/unit_atlas.rs) pre-bakes voxel sprites for every
`(type, facing-bucket, layer, frame, slope_type)` combination via the GPU compute
renderer in [vxl_compute.rs](../../src/render/vxl_compute.rs) (or CPU fallback
[vxl_raster.rs](../../src/render/vxl_raster.rs)). Per frame,
[app_instances/units.rs:121](../../src/app_instances/units.rs#L121) reads the cell's
`slope_type` and picks the matching sprite. The lighting LUT in
[vxl_normals.rs](../../src/render/vxl_normals.rs) is pre-computed per facing-bucket.

The three gaps share one architectural blocker: all of them require **per-tick
variation in the body matrix**. The atlas literally can't represent it — quantizing
even modestly explodes the variant count to millions. Body rocking adds two
continuous-valued axes (roll, pitch). Slope SLERP adds in-flight interpolation between
matrices. Tilt-aware lighting needs the LUT recomputed from the full body matrix.

The current pipeline gracefully handles low-cardinality discrete dimensions (facing
bucket, slope_type) but not per-tick continuous variation.

## Impact Analysis

**Sim-side changes:**
- [sim/components.rs](../../src/sim/components.rs) — new `RockingState` component
- [sim/game_entity.rs](../../src/sim/game_entity.rs) — `rocking: Option<RockingState>` field
- New module `sim/rocking/` with `rocking_system.rs` (per-tick spring-damper)
- [sim/world/mod.rs](../../src/sim/world/mod.rs) `advance_tick` — insert `rocking_system::tick`
  in the standard order (after movement, before render hand-off)

**Rules/INI changes:**
- [rules/warhead_type.rs](../../src/rules/warhead_type.rs) — `rocker: bool`, `direct_rocker: bool`
- [rules/projectile_type.rs](../../src/rules/projectile_type.rs) — `rocker_scale: I8F8`
- [rules/object_type.rs](../../src/rules/object_type.rs) — `weight: SimFixed`
  (parsed from `Weight=`, default 2.0; the L12c divisor in ApplyRocker's force
  formula). Sits on `ObjectType` rather than a per-class struct because the
  TechnoTypeClass base in gamemd parses it for vehicles/aircraft/infantry alike,
  and our `ObjectType` is the equivalent unified type.
- [rules/ruleset.rs](../../src/rules/ruleset.rs) — `direct_rocking_coefficient: I16F16`,
  `fallback_coefficient: I16F16`, `c4_warhead: Option<WarheadKey>` (the
  `[CombatDamage] C4Warhead=` slot at Rules+0xFA8; reused by L30's rocking-tipover
  self-destruct AND by Tanya/SEAL/Engineer C4 detonation — both write the same field)

**Combat-side changes (deferred — depends on combat resolution state):**
- Wherever warhead detonation happens in sim/combat — wire `Rocker=yes` AOE impulse
  and `DirectRocker=yes` direct-hit impulse via new `apply_rocker_impulse` function

**Render-side changes:**
- [render/vxl_compute.rs](../../src/render/vxl_compute.rs) — promote from offline batch
  to per-frame callable; add scratch GPU-texture pool
- [render/vxl_normals.rs](../../src/render/vxl_normals.rs) — new
  `blinn_phong_pages_from_body_matrix` that takes full body matrix
- [render/vxl_raster.rs](../../src/render/vxl_raster.rs) — `VxlRenderParams` gains
  optional `rocking_angles` and `slope_blend_matrix`; `compute_slope_rotation` gains a
  companion `compute_slope_shear_translation` for L24
- [render/unit_atlas.rs](../../src/render/unit_atlas.rs) — passes body matrix (not just
  facing) to LUT pre-compute so atlas variants also get correct lighting per slope
- [app_instances/units.rs](../../src/app_instances/units.rs) — branching: atlas path
  for `rocking.is_neutral()`, real-time path otherwise

**Determinism:** sim state additions all fixed-point. Render-side SLERP and Mat4 math
use `glam` f32 — does not feed back into sim state. State hash includes `RockingState`.

**Blast radius:** Localized. The branching in `units.rs` is a single site. The
real-time render path runs alongside the atlas path; the atlas path is unchanged for
the common case (neutral units).

**Determined to be in-bounds for this design:**
- Closes G1, G3, G4, G7 from the disparity scan
- INI plumbing for Rocker / DirectRocker / RockerScale / DirectRockingCoefficient / FallBackCoefficient

**Determined to be out-of-bounds:**
- The actual call site for `apply_rocker_impulse` in sim/combat is stubbed until
  warhead detonation lands in combat. The rocking machinery sits idle until then —
  no functional gap; just unhooked impulses.
- The vtable[0x298] per-class "should rock?" override in gamemd. **Resolved 2026-05-11
  audit:** gate is data-driven via `TypeClass+0xB0` (single shared implementation across
  all 6 subclasses, NOT virtual override). For Rust we keep the spawn-time assumption
  "vehicles + buildings get `Some(RockingState)`, infantry + aircraft get `None`",
  which approximates the data-driven gate without needing to model TechnoTypeClass+0xB0.
  Revisit only if a specific unit type's rocking looks wrong in-game.

## Tiny-Detail Ledger

Thirty-three items the implementation must preserve. Most recent additions
(2026-05-11 post-audit): L12c (Weight divisor, in-scope for Phase A), L30
(wide-amplitude self-destruct via C4Warhead, in-scope), L31 (distance attenuation,
deferred — documented parity drift), L32 (rate-timer jitter, deferred — documented
parity drift). Sourced from
[BODY_ROCKING_GHIDRA_REPORT.md](../../../ra2-rust-game-docs/BODY_ROCKING_GHIDRA_REPORT.md),
[VXL_DRAW_MATRIX_GHIDRA_REPORT.md](../../../ra2-rust-game-docs/VXL_DRAW_MATRIX_GHIDRA_REPORT.md),
[VXL_HVA_FILE_FORMAT_GHIDRA_REPORT.md](../../../ra2-rust-game-docs/VXL_HVA_FILE_FORMAT_GHIDRA_REPORT.md),
and `rulesmd.ini`.

### Body rocking

- **L1.** Renderer tilt epsilon **0.005 rad**; both axes below → simple matrix path (no tilt math at all). [doc: VXL_DRAW_MATRIX §13]
- **L2.** Per-tick integration: `angle += velocity` each AI tick. [GHIDRA 0x70B570]
- **L3.** Velocity decay (in-range, IsMoving==0): `±0.002 rad/tick`. [GHIDRA constant 0x007F4E70]
- **L4.** Velocity decay (in-range, IsMoving!=0): `±FallBackCoefficient × 0.002 rad/tick`. Default 0.1 → `±0.0002/tick`. [GHIDRA]
- **L5.** Out-of-range push-back: `±0.002 rad/tick` inward, regardless of IsMoving. [GHIDRA]
- **L6.** Snap-back alternative rate: `±0.005 rad/tick` in the velocity-fighting-itself sub-branch. [GHIDRA constant 0x007F4E68 double]
- **L7.** Saturation cap **±π/4** — only when `IsMoving == 0 AND in_range AND prev_angle_was_inside`. Moving vehicles drift past the cap without clamping. [GHIDRA 0x70B789]
- **L8.** Vehicle-crushing-building forwards override: **±π/10** (= 0.31416 rad). Sideways stays at ±π/4. Gate is `TechnoClass+0x6B5 != 0`, set by `DriveLocomotionClass::Process_Drive_Track @ 0x004B1A31` when a Crusher vehicle (TypeClass+0x5B4 == 0xC, TypeClass+0xD2B set) impacts a building. **Plan defers** this override — building-crushing is not implemented, so the gate never fires in practice. Cap stays at ±π/4 for all forwards rocking until crushing lands. [GHIDRA constant 0x007F4E64]
- **L9.** Deadband snap-to-zero: **±2e-5 rad**. Both angle and velocity zeroed. [GHIDRA constants 0x007EC0B0 / 0x007F4E78]
- **L10.** Zero-velocity short-circuit: strict `velocity == 0.0` (not near-zero) → angle force-zeroed. [GHIDRA 0x70B66B]
- **L11.** Apply_area_damage impulse force: `accumulator × 0.01`, saturate at 4.0, gate by `force > 0.3` (`_DAT_007E5138` = 0.3 double, verified 2026-05-11). Per-target 3×3-cell range. [GHIDRA 0x00489D90]
- **L12.** DirectRocker impulse force (vehicle-only): `(RockerScale × Damage >> 8) × DirectRockingCoefficient / 100.0` (`_DAT_0081AEF8` = 100.0 double, verified 2026-05-11; NOT 256 as previously guessed), saturate at 4.0. [GHIDRA 0x00469A50]
- **L12b.** ApplyRocker secondary forwards dampener: `RockingForwardsPerFrame *= 0.5` when `no_dampen == false` (`_DAT_007E5168` = 0.5f, verified 2026-05-11). Sideways velocity is NOT halved — asymmetric. Both DirectRocker and Apply_area_damage call ApplyRocker with `no_dampen == false`, so this halving always applies in practice. [GHIDRA 0x70B54F]
- **L12c.** ApplyRocker per-unit Weight divisor: `force_scaled = (…) × force / TypeClass+0x370` where `TypeClass+0x370 = Weight` (double, default 2.0). Verified 2026-05-11 against `TECHNOTYPECLASS_BASE_ADDENDUM.md:256`. Retail vehicles span Weight 0.5–5 (10× range): light scouts/IFVs ~0.5–1.0, default tanks ~2.0, heavy armor ~3.5–4.0, mammoth-class Apocalypse=5. Heavier units rock proportionally less per equivalent force. Omitting this divisor would make all vehicles rock identically — observable parity drift across mixed armor compositions. [GHIDRA 0x0070B3B8 (`FDIV double ptr [EAX+0x370]`); INI: `Weight=` per-unit in vehicle/aircraft/infantry sections]
- **L31** (deferred — documented parity drift). **ApplyRocker distance attenuation:** `force_scaled = (0.04 − distance × 2.5e-5) × force / Weight` — the impulse scales DOWN linearly with distance from impact source. For Apply_area_damage's 3×3-cell scan, edge cells receive less impulse than center. Constants at 0x007F4E54 (0.04f) and 0x007F4E58 (2.5e-5f), verified 2026-05-11. Smaller parity drift than L12c — distances within a 3×3 cell radius span only ~1.5 cells × 256 lepton/cell ≈ 384 leptons, so the attenuation factor varies from `0.04 − 0` (center) to `0.04 − 0.0096 ≈ 0.0304` (corner cells) = 24% reduction at the corner. Worth implementing for full parity but Phase A can ship without. [GHIDRA 0x0070B394–0x0070B3A1]
- **L32** (deferred — documented parity drift). **ApplyRocker rate-timer jitter:** `angle = (RateTimer − 0x3FFF) × (−π/32768)`; the impulse direction vector is rotated by this angle before per-axis decomposition. The effect is that two identical impacts on identical units at slightly different RateTimer values produce slightly different rocking patterns — the engine intentionally desynchronizes army-of-identical-vehicles rocking so it doesn't look robotic. Omitting it: an army of Apocalypses taking the same artillery shell would rock in perfect lockstep, vs gamemd's subtly-staggered pattern. Visible in mass battles but not in 1v1 micro. [GHIDRA 0x0070B32D–0x0070B358 + `_DAT_007E2810`]
- **L13.** ApplyRocker per-axis velocity saturation: **0.05 rad/tick** max per impulse event. [GHIDRA 0x70B280 inside ApplyRocker]
- **L14.** Defaults: `DirectRockingCoefficient=1.5`, `FallBackCoefficient=0.1`. [ini: rulesmd.ini:620-621]
- **L15.** Per-warhead: `Rocker=no` default (Warhead+0x14E); `DirectRocker=no` default (Warhead+0x14F). [GHIDRA WarheadTypeClass]
- **L16.** Per-bullet: `RockerScale=1.0` Q8.8 default (Bullet+0x150). [doc: BULLETCLASS_LIFECYCLE §7.1]
- **L17.** DirectRocker fires only on vehicles (WhatAmI == 1); infantry/buildings/aircraft fall through to area path. [GHIDRA 0x469A0C]
- **L30.** Wide-amplitude self-destruct: when `|AngleRotatedSideways| > π` OR `|AngleRotatedForwards| > π` at end of RockingUpdate, the unit calls `TechnoClass::ReceiveDamage` on itself with `damage = TypeClass+0xA0` (max-HP-class scalar), `warhead = Rules.c4_warhead` (the `[CombatDamage] C4Warhead=` field, which retail sets to `Super` — designer-annotated as the "Absolute damage" warhead), `force_kill = 1` (bypass armor multiplier and veterancy adjustments). The unit dies. Retail YR almost never reaches this path in skirmish (the constants and per-type `TypeClass+0xD6A` ship-rock-clamp flag are tuned so legitimate impulses stay within ±π/4); the path exists as a safety net for catastrophic states (sustained EMP on a type without the ship-rock clamp, stacked rocker impulses on a moving vehicle, external angle writes). Faithful port must include it for parity completeness — modded warheads or edge-case states would otherwise diverge. [GHIDRA 0x0070BC23 (trigger) → 0x00701900 (ReceiveDamage); INI: `[CombatDamage] C4Warhead=Super`; constants: ±π = `_DAT_007F4E5C` / `_DAT_007F4E60`]

### Slope-tilt SLERP

- **L18.** Transition duration: hard-coded **3 ticks**. [doc: VXL_DRAW_MATRIX §12]
- **L19.** Interpolation: genuine quaternion SLERP with three cases (near-identical, near-opposite, normal-omega formula). [doc: VXL_DRAW_MATRIX §2.1]
- **L20.** Fraction: `(3 − remaining) / 3`. [doc: §12]
- **L21.** State per locomotor: `prev_slope`, `curr_slope`, `transition_start_frame`, `duration=3`. [doc: §12]
- **L22.** Transition fires when `prev_slope != curr_slope` on cell change. [doc: §12]
- **L23.** Aircraft skip slope tilt entirely (forced slope=0). [doc: VOXEL_SLOPE_TILT §3]
- **L24.** Slope-tilt translation shear offsets (`combined_Z`, `partial_X/Y`, `remainder_X/Y`) keep the rotated body sitting visually on the slope. [doc: VXL_DRAW_MATRIX §15.2-§15.4]

### Lighting LUT

- **L25.** LUT recomputed per Render call (per-techno-per-frame), not per facing-bucket. [doc: VXL_HVA §6.4]
- **L26.** World-space light: constant `(-0.7071, -0.7071, 0)`. [doc: §6.5]
- **L27.** LUT inputs: `dot(local_normal, body_matrix⁻¹ × world_light)` — the FULL body matrix (facing × slope × rocking). [doc: §6.4 derivation]
- **L28.** Specular strength **3.0** (already correct in Rust). [doc: §1 #3]
- **L29.** Brightness × 16 → page index. [doc: §6.4]

## Chosen Approach

**Approach A — Hybrid atlas + real-time render**, chosen 2026-05-11 over two
alternatives (shader-warp 2D approximation, hybrid-with-SLERP-dropped). See
"Alternatives Considered" at the bottom.

Sim-side: a new fixed-point spring-damper system advances rocking angles each tick.
Render-side: at the atlas-lookup site, units with `RockingState::is_neutral() == true`
take the existing atlas path; units with active rocking or in-progress slope
transition take a real-time render via `vxl_compute.rs`, with the full body matrix
computed per frame. Lighting LUT is recomputed from the body matrix on both paths
(closing G4 for upright-on-slope as well as actively-tilting units).

This satisfies all 29 ledger items by construction. The two architectural costs are
(1) a branching point in the unit-draw loop and (2) `vxl_compute.rs` being promoted
from offline-batch to per-frame use. Performance at 20k-unit scale is the largest
open risk; the design includes a fallback plan (LRU-cap real-time renders; remainder
fall back to atlas with stale slope angles) that bounds the parity drift if perf
doesn't hold up.

## Design

### Components

```rust
// src/sim/components.rs
use fixed::types::I16F16;

#[derive(Debug, Clone, Copy, Default, serde::Serialize, serde::Deserialize)]
pub struct RockingState {
    /// Roll, rad. Sign convention matches gamemd's AngleRotatedSideways.
    pub angle_sideways: I16F16,
    /// Pitch, rad. Sign convention matches gamemd's AngleRotatedForwards.
    pub angle_forwards: I16F16,
    /// Angular velocity, rad/tick.
    pub vel_sideways: I16F16,
    pub vel_forwards: I16F16,
    /// IsShipRocking gate — when true, integrate without damping. Used by EMP
    /// wobble and naval rocking. Set externally by EMP / naval-impact code;
    /// cleared when those external states clear.
    pub is_ship_rocking: bool,
    /// Slope before the current transition (== curr_slope when no transition).
    pub prev_slope: u8,
    /// Cell's current slope_type.
    pub curr_slope: u8,
    /// Counts down from 3 to 0. Nonzero ⇒ SLERP between prev_slope and curr_slope.
    pub transition_ticks_remaining: u8,
}

impl RockingState {
    pub fn is_neutral(&self) -> bool {
        let deadband = I16F16::from_num(2e-5);
        self.angle_sideways.abs() <= deadband
            && self.angle_forwards.abs() <= deadband
            && self.transition_ticks_remaining == 0
            && !self.is_ship_rocking
    }
}
```

Add `rocking: Option<RockingState>` to `GameEntity`. Populated at spawn for vehicles,
ships, and voxel-bodied buildings; `None` for infantry, aircraft, and SHP-bodied
buildings.

### Sim system

```rust
// src/sim/rocking/rocking_system.rs

const TILT_DEADBAND: I16F16     = /* from_num(2e-5) */;
const SATURATION_PI4: I16F16    = /* from_num(0.78539816) */;
const SATURATION_PI10: I16F16   = /* from_num(0.31415926) */;
const NORMAL_RANGE_PI2: I16F16  = /* from_num(1.57079632) */;
const BASE_DECAY_RATE: I16F16   = /* from_num(0.002) */;
const SNAP_BACK_RATE: I16F16    = /* from_num(0.005) */;
const IMPULSE_VEL_CAP: I16F16   = /* from_num(0.05) */;
const SINK_TILT_PER_TICK: I16F16 = /* from_num(0.01) */;

pub fn tick(world: &mut World) {
    for (_id, entity) in world.entities.iter_mut() {
        let Some(rocking) = entity.rocking.as_mut() else { continue };

        // (1) Slope-transition tick-down (L18-L22).
        let cell_slope = cell_slope_at(&world.map, entity.position).unwrap_or(0);
        let cell_slope = if entity.is_aircraft() { 0 } else { cell_slope };  // L23
        if cell_slope != rocking.curr_slope {
            rocking.prev_slope = rocking.curr_slope;
            rocking.curr_slope = cell_slope;
            rocking.transition_ticks_remaining = 3;
        } else if rocking.transition_ticks_remaining > 0 {
            rocking.transition_ticks_remaining -= 1;
        }

        // (2) Spring-damper advance.
        if rocking.is_ship_rocking {
            advance_ship_rocking(rocking, entity);
            continue;
        }
        let is_moving = entity_is_moving(entity);
        let fallback = world.rules.fallback_coefficient;
        advance_axis_sideways(rocking, is_moving, fallback);
        advance_axis_forwards(rocking, entity, is_moving, fallback);
    }
}

fn advance_axis(angle: &mut I16F16, velocity: &mut I16F16, cap: I16F16,
                is_moving: bool, fallback: I16F16) {
    // L10: strict velocity == 0 → angle force-zero.
    if *velocity == I16F16::ZERO {
        *angle = I16F16::ZERO;
        return;
    }

    // L2: integrate.
    let prev = *angle;
    let new_angle = prev + *velocity;
    *angle = new_angle;

    let in_range = angle.abs() <= NORMAL_RANGE_PI2;

    // L7: saturation only when NOT moving AND in normal range AND crossing the boundary.
    if !is_moving && in_range {
        if new_angle > cap && prev < cap {
            *angle = cap;
            *velocity = I16F16::ZERO;
        } else if new_angle < -cap && prev > -cap {
            *angle = -cap;
            *velocity = I16F16::ZERO;
        }
    }

    // L3/L4/L5: dampening.
    let decay = if is_moving { fallback * BASE_DECAY_RATE } else { BASE_DECAY_RATE };
    if *velocity > I16F16::ZERO {
        if in_range { *velocity -= decay } else { *velocity -= BASE_DECAY_RATE };  // out-of-range push back inward
    } else if *velocity < I16F16::ZERO {
        if in_range { *velocity += decay } else { *velocity += BASE_DECAY_RATE };
    }

    // L9: deadband snap-to-zero.
    if angle.abs() <= TILT_DEADBAND {
        *angle = I16F16::ZERO;
        *velocity = I16F16::ZERO;
    }
}

pub fn apply_rocker_impulse(
    entity: &mut GameEntity,
    source_pos: Vec3Fixed,
    force: I16F16,
    no_dampen: bool,
) {
    // Port of TechnoClass::ApplyRocker (FUN_0070B280).
    // Computes direction-aware velocity components, saturates per-axis at 0.05 rad/tick.
    // Sets entity.rocking.vel_sideways and vel_forwards.
    // ...
}
```

The ship-rocking path is structurally simpler (integrate + clamp, no damping):

```rust
fn advance_ship_rocking(rocking: &mut RockingState, entity: &GameEntity) {
    rocking.angle_forwards += rocking.vel_forwards;
    rocking.angle_sideways += rocking.vel_sideways;
    if !entity.type_supports_ship_rocking() {  // TypeClass+0xD6A gate (per RE doc)
        return;
    }
    // One-sided lower clamps + upper sideways clamp.
    rocking.angle_forwards = rocking.angle_forwards.max(-SATURATION_PI4);
    rocking.angle_sideways = rocking.angle_sideways.max(-SATURATION_PI4);
    if rocking.angle_sideways >= SATURATION_PI4 {
        rocking.angle_sideways = SATURATION_PI4;
    }
}
```

**Tick ordering:** insert `rocking_system::tick` into `World::advance_tick` after the
movement phase and before the render-hand-off phase. Rocking must see the latest cell
position to read the correct slope_type.

### Interfaces / Contracts

`RockingState::is_neutral() -> bool` — the gate the renderer uses to decide between
atlas and real-time paths.

```rust
// src/render/vxl_normals.rs (new)
pub fn blinn_phong_pages_from_body_matrix(normals_mode: u8, body_matrix: &Mat4) -> [u8; 256];
```

```rust
// src/render/vxl_compute.rs (modified)
impl VxlComputeRenderer {
    pub fn render_runtime(&mut self,
        vxl: &VxlFile, hva: Option<&HvaFile>,
        body_matrix: Mat4, vpl: Option<&VplFile>,
    ) -> RuntimeSprite;
    // Outputs to a scratch GPU texture allocated from an internal pool.
    // RuntimeSprite holds: texture handle + UV + offset + size.
}
```

```rust
// src/render/vxl_raster.rs (modified)
pub struct VxlRenderParams {
    // existing fields …
    pub rocking_angles: Option<(f32, f32)>,  // (sideways, forwards) in rad. None ⇒ no rocking.
    pub slope_blend_matrix: Option<Mat4>,    // Pre-SLERPed slope matrix. None ⇒ use slope_type directly.
}
```

### Data flow

```
[sim/world tick]
  → movement_system (updates positions)
  → rocking_system::tick
      ├─ reads cell.slope_type at each rockable entity
      ├─ advances slope-transition state
      └─ advances spring-damper angles/velocities

[sim/combat warhead detonation — DEFERRED]
  → if warhead.rocker: for each 3×3-cell target
      → apply_rocker_impulse(target, source_pos, force, no_dampen=false)
  → if warhead.direct_rocker && target.is_vehicle:
      → apply_rocker_impulse(target, target_pos+offset, force, no_dampen=false)

[render hand-off in app_instances/units.rs]
  → for each entity:
      if rocking.is_neutral():
          [atlas path — UNCHANGED]
          → atlas.lookup(UnitSpriteKey { type, facing, layer, frame, slope_type })
          → draw quad
      else:
          [real-time path]
          → compute_body_matrix(facing, slope_blend_via_SLERP, rocking_angles)
          → blinn_phong_pages_from_body_matrix(normals_mode, body_matrix)
          → vxl_compute.render_runtime(vxl, hva, body_matrix, vpl)
          → draw quad referencing scratch texture
```

### Error handling

- Slope-type out of range (>16): clamp to 0 (matches existing
  `app_instances/units.rs` tripwire behavior).
- `compute_body_matrix` invalid inputs (e.g., NaN from upstream): defensive fall-through
  to identity body matrix; logged once.
- `vxl_compute.render_runtime` scratch-texture-pool exhaustion: bump pool size; never
  fail silently. If we hit the perf cap, fall back to atlas with stale-slope drift.
- INI defaults applied if any field missing (per L14, L15, L16).

### Testing strategy

**Unit tests (sim/rocking):**
1. `spring_damper_convergence` — apply 0.05 rad/tick velocity impulse; verify angle
   decays to <2e-5 within ~250 ticks (default FallBack=0.1, IsMoving=false).
2. `saturation_cap_stationary_only` — stationary unit + impulse → angle clamps at π/4
   and velocity zeroes. Moving unit + same impulse → angle exceeds π/4 (no clamp).
3. `deployed_building_forwards_uses_pi10` — building entity with deployed=true clamps
   forwards at π/10, sideways at π/4.
4. `deadband_snap` — angle drifting through ±2e-5 zeroes both angle and velocity in
   the same tick.
5. `velocity_exactly_zero_force_zeros_angle` — strict `== 0`, not near-zero.
6. `ship_rocking_no_damping` — `is_ship_rocking=true` integrates velocity for many ticks
   without decay; clamps once at the ±π/4 boundary.
7. `slope_transition_three_tick_countdown` — slope change → ticks_remaining=3; after 3
   sim ticks, back to 0.

**Determinism tests:**
- Run same world twice for 1000 ticks with same initial rocking impulses; state hash
  matches tick-by-tick.

**Integration tests:**
- Synthetic map with a slope boundary; place a vehicle, drive across; capture render
  output at ticks 0/1/2/3/4. Confirm SLERP intermediate matrices match expected blend.
- Spawn a vehicle, fire an `apply_rocker_impulse` directly; capture render at ticks
  0/10/50/100/250; confirm decay matches gamemd reference visually.

**Performance benchmark:**
- Synthetic stress: 2000 rocking entities in a single frame. Measure GPU compute time
  per frame. Validates the 10%-of-20k assumption.

### Determinism considerations

- All sim state is fixed-point. Spring-damper math is `I16F16` arithmetic — bit-exact
  across replays.
- Render-side SLERP uses `glam::Quat::slerp` (f32) but does not feed back into sim
  state — only into the body matrix used for that frame's draw.
- State hash includes `RockingState` per entity.
- Slope-transition state (`prev_slope`, `curr_slope`, `transition_ticks_remaining`) is
  integer — trivially deterministic.

## Architectural Decisions

**Patterns followed:**
- New sim system in `src/sim/rocking/` mirrors the structure of existing sim systems
  (`miner`, `pathfinding`, `combat`).
- INI field additions follow the existing `WarheadType` / `ProjectileType` / `Ruleset`
  shape — no new abstraction.
- Optional component on `GameEntity` matches existing optional-component pattern
  (e.g., `HarvestOverlay`).
- Tick-ordering insertion into `World::advance_tick` follows the documented standard
  order.

**Patterns deviated from:**
- The renderer gets a *branching path* (atlas vs real-time) at the unit-draw site.
  Today the unit-draw path is a single atlas lookup. This adds complexity but is the
  only way to satisfy the parity bar at the body-matrix level.
  - Justification: the atlas can't represent continuous-valued body-matrix variation.
    The branching is localized to one site.
- `vxl_compute.rs` changes from offline batch to per-frame callable. Existing batch
  use during atlas bake stays — we just add a new entry point that submits per-frame
  without CPU readback.
  - Justification: the GPU compute renderer already does the right thing math-wise;
    we just need to plumb it for per-frame use.

**Tech debt introduced:**
- Performance at 20k-unit scale is the largest unknown. The design includes a
  fallback plan (LRU-cap real-time renders, atlas fallback with stale slope for the
  overflow) but if it triggers, we have a documented parity drift on the
  capped-overflow units. Plan to address: benchmark early during implementation; if
  the naive design doesn't hold, batch all real-time renders into one compute
  dispatch per frame before considering the LRU cap.
- The `vtable[0x298]` per-class rocking gate from gamemd is approximated as
  "vehicles + buildings rock; infantry + aircraft don't" until RE confirms. If we
  find a class that flips this assumption, fix is a 1-line edit to the
  `GameEntity` spawn site.

## Alternatives Considered

**Approach B — Shader-warp 2D approximation.** Apply roll/pitch as a 2D affine warp
in the sprite vertex shader. Trivial perf at any scale, but observably wrong:
silhouette of a 2D-warped flat sprite ≠ true 3D voxel rotation, turret pivot is
glued to the body, and lighting LUT can't follow the tilt without per-unit recompute
(which kills the supposed perf benefit). Three distinct parity drifts. Rejected.

**Approach C — Hybrid with SLERP dropped.** Approach A but revert slope transition
to a 1-tick snap. Smaller surface area to implement but introduces a known
player-visible drift on every slope-boundary crossing — common, several times per
match per unit. Per CLAUDE.md severity rule, this is convenience-disguised parity
drift with no valid scope-cut justification (not prerequisite-blocked, not TS-legacy).
Rejected.

**Approach A' — Drop the atlas entirely; always real-time.** Simpler architecture
(one render path) but the existing `vxl_compute.rs` is "offline batch tool, not
per-frame" — at 20k units this is significant perf risk with no fast path for the
common case (most units neutral). Approach A's hybrid keeps the atlas's amortized
cost for the common case and pays the per-frame cost only for actively-tilting
units. Worth revisiting if benchmarking shows real-time-always is fast enough.

## Open Questions

Carried forward from RE. **2026-05-11 verify-doc audit resolved Q1–Q3 — all values
verified from binary.** None block implementation.

1. ~~`vtable[0x298]` per-class behavior.~~ **RESOLVED:** Not per-class polymorphic. All
   6 TechnoClass subclasses share one implementation at 0x006F9E10 returning
   `(*(byte *)(TypeClass+0xB0) == 0)`. Rocking is data-driven via TypeClass+0xB0.
   Our spawn-time `Option<RockingState>` decision approximates this. Revisit only if
   a specific unit type's rocking looks wrong in-game.
2. ~~The Apply_area_damage force-floor constant `_DAT_007E5138`.~~ **RESOLVED: 0.3
   double.** Gate is `force > 0.3` (after the 4.0 saturation). Use this in L11
   implementation, not `force > 0.0`.
3. ~~The DirectRocker normalization constant `_DAT_0081AEF8`.~~ **RESOLVED: 100.0
   double, NOT 256.** Update L12 implementation to divide by 100.0. Implementing as
   256 (the earlier Q8.8 guess) would produce force impulses ~2.56× smaller than retail.
4. Where IsShipRocking (+0x425) gets cleared after EMP expiry. Working assumption:
   wired in the EMP-tick-expiry system when it lands. (Still open.)
5. **NEW:** ApplyRocker secondary forwards dampener `_DAT_007E5168` = 0.5f, verified
   2026-05-11. Forwards velocity is halved when `no_dampen_flag == false`; sideways is
   not halved. L12b in the ledger captures this.

## Hand-Off

This design is ready for `/write-plan`. The plan should break implementation into:
1. Sim-side: `RockingState` + `rocking_system` + INI plumbing (mostly self-contained,
   testable without renderer changes).
2. Render-side: body matrix composition + `vxl_compute` per-frame promotion + LUT
   refactor (depends on 1 for the angle inputs).
3. Atlas path lighting upgrade — pass body matrix into LUT pre-compute (small, can
   land alongside 2).
4. Combat-side impulse wiring (deferred until warhead detonation lands in sim/combat).
