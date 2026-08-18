# VXL Slope Tilt Constants Design

## Goal

Replace the heuristic edge/corner tilt constants in the voxel renderer with the
binary-verified gamemd.exe values, so vehicles on sloped cells (slope_type 1-8)
lean by the same magnitude as in the original game.

## Architecture Context

The voxel slope-tilt pipeline is already wired end-to-end in the renderer:

- `cell.slope_type: u8` produced by [src/map/resolved_terrain.rs:1132](src/map/resolved_terrain.rs#L1132)
  from the .map tile data byte at offset +0x2A.
- [src/app_instances/units.rs:78-87](src/app_instances/units.rs#L78-L87) reads the
  cell's slope_type per-frame, clamps to 0-8 (rejecting unimplemented 9-20), and
  passes it through `UnitSpriteKey`. Aircraft are forced to `slope_type=0`.
- [src/render/unit_atlas.rs](src/render/unit_atlas.rs) keys atlas entries by
  `(type_id, facing, house_color, layer, frame, slope_type)`, so each unit gets
  one pre-rendered tilted variant per slope.
- [src/render/vxl_raster.rs:249-267](src/render/vxl_raster.rs#L249-L267)'s
  `compute_slope_rotation(slope_type)` builds `Rz(compass) × Rx(tilt) × Rz(-compass)`,
  matching gamemd's `FUN_005AE6F0` rotate-tilt-unrotate pattern.
- [src/render/vxl_raster.rs:327-328](src/render/vxl_raster.rs#L327-L328) applies it
  in `prepare_limb_data`: `combined = rotate_to_world * slope_mat * section_transform`.
- The GPU compute path ([src/render/vxl_compute.rs](src/render/vxl_compute.rs))
  shares `prepare_limb_data`, so the same constants drive both paths.

The only thing wrong is the magnitude of the two `const` values themselves.

## Impact Analysis

**Touches:** `src/render/vxl_raster.rs` (two `const` values + their docstrings,
plus one new test).

**Doesn't touch:** sim/, asset loaders, GPU shaders, atlas key schema, app
instance plumbing. Behavior change is entirely captured in the two numeric
literals.

**Blast radius:** every voxel unit on a slope_type 1-8 cell renders with a
stronger lean (~35% more for edge ramps, ~37% more for corner ramps). No
sim-side state changes; lockstep hash unaffected.

**Cache concern:** the unit atlas pre-renders tilted variants. After the
constant change the atlas needs to be rebuilt, which it does on app start
(no on-disk cache for unit-atlas tilted entries to invalidate).

**Determinism:** render-only. World state hash unaffected.

## Chosen Approach

Approach A — change the two constants, update their docstrings to cite the
gamemd formula instead of the prior heuristic, and add one geometry test
asserting that the tilt direction matches gamemd's compass convention.

Why over the alternatives:

- **Wider scope (slopes 9-20 / dynamic roll-pitch / aircraft turret states)** is
  blocked on prerequisites that don't exist yet (matrix entries for 9-20 require
  a separate Ghidra pass; dynamic roll-pitch needs sim-side integration; aircraft
  state machine needs RE on `FUN_00729B40`). Each is queued as a named follow-up
  rather than guessed at.
- **Pre-baking the slope matrix table** (gamemd-style cache at `DAT_00B45188`)
  is a perf optimization with no parity benefit. YAGNI; the matrices are computed
  once per atlas entry, not per pixel.

## Tiny-Detail Ledger

| # | Detail | Source | Where it lives in the design |
|---|---|---|---|
| 1 | `EDGE_TILT_RAD = atan(2 × 104 / 256√2) ≈ 0.5214767 rad` | `[GHIDRA 0x00754A50]` chain | New `const` value |
| 2 | `CORNER_TILT_RAD = atan(104 / 256) ≈ 0.3858827 rad` | `[GHIDRA 0x00754A20]` + `[GHIDRA 0x007E1740]` | New `const` value |
| 3 | Slope matrix construction: `Rz(compass) × Rx(tilt) × Rz(-compass)` | `[doc §Matrix Construction]` | Already in `compute_slope_rotation`; unchanged |
| 4 | Final body matrix: `result = facing × slope` (slope applied to model BEFORE facing rotates it) | `[doc §Two Separate Tilt Paths]` | Already in `prepare_limb_data:328`; unchanged |
| 5 | Compass directions: W=270°, N=180°, E=90°, S=0°, NW=225°, NE=135°, SE=45°, SW=315° | `[doc §Slope Type Values]` | Already in `compute_slope_rotation`'s match arms; unchanged |
| 6 | Slopes 1-4 use `EDGE_TILT_RAD`; 5-8 use `CORNER_TILT_RAD` | `[doc §Slope Type Values]` | Already in code; unchanged |
| 7 | Slope tilt is visual only — no sim effect | `[doc §Overview]` | Renderer has no sim back-channel; structurally enforced |
| 8 | Aircraft skip slope tilt entirely (`slope_type = 0`) | `[code units.rs:78-79]` | Already in code; unchanged |
| 9 | `LevelHeight = 104 leptons` is the rise constant the formulas reduce around | `[GHIDRA 0x007549E0]` | Captured in new docstring as the formula's anchor |
| 10 | Sign convention: positive tilt + slope_type 4 (south compass=0°) ⇒ south side rises in projected screen Y | `[UNKNOWN — needs test]` | New `test_slope_4_south_corner_rises` test |
| 11 | Slopes 9-20 tilt matrices | `[UNKNOWN — needs RE: trace FUN_00754CB0 for indices 9-20]` | **Deferred follow-up** — see below |
| 12 | Dynamic roll/pitch from acceleration (entity+0x328/0x32C) | `[doc §Vehicle Body Tilt]` | **Deferred follow-up** — needs sim-side work |
| 13 | Aircraft turret state machine (states 2-7) | `[doc §Turret/Barrel Tilt]` | **Deferred follow-up** — needs RE on `FUN_00729B40` |
| 14 | glam right-handed `from_rotation_x` matches gamemd's tilt sign | `[UNKNOWN — needs test]` | Same test as item 10 |

## Design

### Components

A single file change, no new modules.

```rust
// src/render/vxl_raster.rs

/// Edge ramp tilt angle (slope types 1-4): atan(2 × LevelHeight / cellDiagonal).
///
/// Derived in gamemd by `VXL_Init_EdgeTiltAngle` (0x00754A50):
///     atan(2 × DAT_00B45578 / DAT_00B43EF8)
/// where DAT_00B45578 = 104 leptons (LevelHeight) and DAT_00B43EF8 = 256√2
/// leptons (cell diagonal). Equals atan(13√2/32) = 0.5214767 rad ≈ 29.88°.
const EDGE_TILT_RAD: f32 = 0.521_540_3;

/// Corner ramp tilt angle (slope types 5-8): atan(LevelHeight / cellSide).
///
/// Derived in gamemd by `VXL_Init_CornerTiltAngle` (0x00754A20):
///     atan(DAT_00B45578 / 256)
/// where DAT_00B45578 = 104 leptons (LevelHeight) and 256 = cellSide leptons.
/// Equals atan(13/32) = 0.3858827 rad ≈ 22.10°.
const CORNER_TILT_RAD: f32 = 0.385_866_0;
```

### Interfaces / Contracts

No public API changes. `compute_slope_rotation`'s return shape unchanged
(`Mat4`); `prepare_limb_data`'s signature unchanged. The atlas cache key
schema (`UnitSpriteKey`) is unchanged.

### Data Flow

Unchanged. `cell.slope_type → UnitSpriteKey → prepare_limb_data →
compute_slope_rotation → combined Mat4 → rasterizer/GPU compute`.

### Error Handling

No new error paths. `compute_slope_rotation` already returns `Mat4::IDENTITY`
for slope_type 0 and unknown values (9-20).

### Testing Strategy

Add one geometry test in `src/render/vxl_raster.rs`'s existing `tests` module:

```rust
#[test]
fn test_slope_4_south_high_corner_rises() {
    // Slope type 4 = South: south corners raised. After applying the slope
    // matrix to a unit voxel pointing in +Y (model "north"), the projected
    // screen Y must DECREASE (north voxel projects up-screen) more than
    // a south-pointing voxel. This catches sign/handedness regressions
    // in either the constants or glam's rotation convention.
    let slope_mat = compute_slope_rotation(4);

    // +Y in model space, mapped through slope only (no facing/camera).
    let north = slope_mat.transform_point3(Vec3::Y);
    let south = slope_mat.transform_point3(-Vec3::Y);

    // For slope type 4 (south corners high), south.z > north.z.
    assert!(
        south.z > north.z,
        "Expected south corner to be higher than north for slope_type=4; \
         got north.z={}, south.z={}",
        north.z, south.z
    );
}

#[test]
fn test_edge_tilt_magnitude_matches_gamemd() {
    // Sanity: the constant equals atan(2 * 104 / (256 * sqrt(2))) within f32 epsilon.
    let expected: f32 = (2.0_f32 * 104.0 / (256.0 * 2.0_f32.sqrt())).atan();
    assert!((EDGE_TILT_RAD - expected).abs() < 1e-5);
}

#[test]
fn test_corner_tilt_magnitude_matches_gamemd() {
    let expected: f32 = (104.0_f32 / 256.0).atan();
    assert!((CORNER_TILT_RAD - expected).abs() < 1e-5);
}
```

The first test catches sign/handedness errors (ledger items 10 & 14). The other
two are tripwires that prevent silent rot if someone copy-pastes the wrong
literal in future.

A separate visual smoke check is recommended at runtime: load a map with
clearly-visible ramps (e.g. one of the cliff-edge skirmish maps), drop a Rhino
on each of slopes 1-8, and eyeball-compare against a gamemd screenshot. Not
automated.

## Architectural Decisions

- **No deviation from existing patterns.** Constants live alongside their
  consumer, formulas are documented in docstrings, tests live in the same
  file's `tests` module — exactly the conventions the file already uses.
- **No new abstraction.** Keeping the slope matrices computed on demand in
  `compute_slope_rotation` rather than introducing a startup-time cache table
  (gamemd-style `DAT_00B45188`) — the perf wins don't matter at our scale.
- **Tech debt: none introduced.** The deferred follow-ups (slopes 9-20,
  dynamic roll/pitch, aircraft turret states) are pre-existing gaps,
  not new ones.

## Deferred Follow-Ups (named, not dropped)

These are the parity gaps NOT closed by this change. Each is blocked on a
prerequisite that doesn't exist yet:

1. **Slopes 9-20 (mid/steep/double ramps).** Renders flat today. Needs
   either a Ghidra pass through `FUN_00754CB0`'s init code for indices 9-20,
   or a runtime debugger snapshot of `DAT_00B45188 + n*0x30` for those
   indices. Visible on cliff-transition cells in standard YR maps.

2. **Dynamic body roll/pitch from acceleration/braking.** Needs sim-side
   work: driving locomotor must compute roll/pitch into entity state
   (mirroring gamemd's `entity+0x328`/`+0x32C`), and `FUN_005AEF60`/
   `FUN_005AF080` need RE for the exact compute. Visible on every
   accelerating tank.

3. **Aircraft turret state machine (states 2-7).** `FUN_00729B40`'s
   takeoff/hover/descend/land state transitions need RE for transition
   timing and progress curves. Visible on Kirov/Rocketeer/Harrier/Black
   Eagle/Siege Chopper takeoffs and landings.

## Alternatives Considered

- **Approach B — extract slopes 9-20 in same session.** Rejected: those are
  matrices, not single tilt angles, and the gamemd init code path for them
  hasn't been traced. Risk of publishing wrong values for 12 slope types.
- **Approach C — pre-bake slope matrix table at startup.** Rejected as YAGNI.
  Pure perf optimization with no parity benefit; matrices are already
  computed at atlas-build time (once per entry), not per pixel.
