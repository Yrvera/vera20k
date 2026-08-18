# VXL Slope Transition Design

## Goal

Render the standard YR 3-frame vehicle voxel slope transition so units visually blend from the previous terrain slope to the current terrain slope instead of snapping between discrete slope sprites.

## Architecture Context

Vehicle VXL sprites are currently pre-rendered into `UnitAtlas` by `src/render/unit_atlas.rs`, keyed by type, facing, layer, HVA frame, and one discrete `slope_type`. `src/app_instances/units.rs` samples `ResolvedTerrainCell.slope_type` at the entity's current cell and emits `UnitSpriteKey` entries for body/composite/turret/barrel.

`src/sim/rocking/rocking_system.rs` already updates `RockingState.prev_slope`, `curr_slope`, and `transition_ticks_remaining` from the current occupied cell. Existing comments correctly point at a render consumer, but the app/render path currently ignores those fields.

Recent targeted research resolves the behavior:

- `VXL_SLOPE_CELL_SAMPLING_GHIDRA_REPORT.md`: `DriveLocomotionClass::Process @ 0x004B0500` samples the owner's current occupied cell, reads `CellClass+0x11C`, caches previous/current slope, and starts a 3-frame timer on change.
- `VXL_DRAW_MATRIX_ORDER_GHIDRA_REPORT.md`: simple vehicle VXL transform is `camera_view * slope_matrix * facing_rotation * section_or_hva_matrix`.
- `VXL_SLOPE_MATRIX_SIGN_GHIDRA_REPORT.md`: the existing positive slope matrix sign is correct; do not negate `tilt_rad`.

## Impact Analysis

Touched ownership:

- `src/sim/rocking/rocking_system.rs`: existing slope-transition state remains the source of truth. No new dependency on render.
- `src/render/vxl_raster.rs`: add a render parameter capable of expressing a blended previous/current slope matrix at a normalized transition phase.
- `src/render/unit_atlas.rs`: keep the permanent atlas for stable discrete sprites; do not pre-bake every slope pair globally.
- `src/app_instances/units.rs`: choose the permanent atlas path when no transition is active; use a transient transition sprite path while `transition_ticks_remaining > 0`.

Risks:

- Atlas explosion if every slope-pair phase is pre-baked globally.
- Cache churn if transition sprites are rebuilt every frame with no reuse.
- Visual mismatch if the phase formula is off by one.
- Layer mismatch if body/turret/barrel do not share the same transition matrix.

## Chosen Approach

Use a transient VXL slope-transition cache for only the slope-pair phases that are actually needed by visible entities.

Permanent atlas entries remain keyed by one `slope_type` and are used for normal rendering. During the 3-frame slope transition, `app_instances/units.rs` requests a transient sprite keyed by:

```text
type_id + facing + layer + frame + prev_slope + curr_slope + phase
```

The transient sprite is generated through the same VXL raster pipeline, but with a blended slope matrix instead of a single discrete `slope_type`. It is cached for reuse during the session and can be stored outside the permanent atlas to avoid repacking the main atlas during gameplay.

## Tiny-Detail Ledger

- Current occupied cell supplies slope, not destination/next path cell. Source: `VXL_SLOPE_CELL_SAMPLING_GHIDRA_REPORT.md`, `DriveLocomotionClass::Process @ 0x004B0510..0x004B051B`.
- Slope change stores old current slope as previous and new sampled slope as current. Source: `VXL_SLOPE_CELL_SAMPLING_GHIDRA_REPORT.md`, `0x004B0523..0x004B0533`.
- Transition duration is exactly 3 frames. Source: `VXL_SLOPE_CELL_SAMPLING_GHIDRA_REPORT.md`, `CDTimerClass__Start(3) @ 0x004B052D..0x004B0536`.
- Draw reads cached previous/current slope and timer fields; it does not sample terrain directly. Source: `VXL_SLOPE_CELL_SAMPLING_GHIDRA_REPORT.md`, `DriveLocomotionClass::Draw_Matrix @ 0x004AFF60`.
- If transition is complete, direct current slope lookup is used. Source: `VXL_SLOPE_CELL_SAMPLING_GHIDRA_REPORT.md`, direct `VXL_GetFacingMatrix @ 0x004B0390..0x004B03A0`.
- Slope interpolation uses `t = (total - remaining) / total`; it reaches direct current slope when `t >= 1.0`. Source: `VXL_SLOPE_CELL_SAMPLING_GHIDRA_REPORT.md`, `0x004B01B2..0x004B01DE`, `0x004B0351..0x004B0371`.
- Simple transform order is `camera_view * slope_matrix * facing_rotation * section_or_hva_matrix`. Source: `VXL_DRAW_MATRIX_ORDER_GHIDRA_REPORT.md`, `0x004B03CB..0x004B03DA`, `0x00706FCE`.
- Slope matrix sign is positive `Rx(tilt)` as currently implemented. Source: `VXL_SLOPE_MATRIX_SIGN_GHIDRA_REPORT.md`, `Matrix_rotate_x_axis @ 0x005AEF60`.
- Slope types 17-20 remain flat/clamped in Rust by existing defensive policy. Source: `VOXEL_SLOPE_TILT_SYSTEM.md` and current `app_instances/units.rs` clamp.

## Design

### Components

1. `VxlRenderParams` gains optional slope-transition data:

```text
slope_blend: None
  -> existing discrete slope_type behavior

slope_blend: Some { from_slope, to_slope, phase }
  -> compute blended slope matrix, then compose camera * blended_slope * facing * section
```

2. `vxl_raster.rs` owns matrix generation:

- `compute_slope_rotation(slope_type)` remains unchanged.
- Add `compute_slope_transition_rotation(from, to, phase)`.
- Use quaternion slerp or a verified equivalent over the two slope rotations, then convert back to `Mat4`.

3. `app_instances/units.rs` owns path choice:

- Aircraft: always slope 0, no transition.
- Ground voxel unit without active transition: use permanent atlas with `curr_slope`.
- Ground voxel unit with active transition: request transient transition sprite for the relevant phase.

4. Add a small render-owned transition sprite cache:

- Key mirrors `UnitSpriteKey` plus `from_slope`, `to_slope`, and phase.
- Value is GPU texture/UV data compatible with `SpriteInstance`.
- Cache is bounded or session-scoped; stale transition entries can be retained initially because only `17 * 17 * 3` slope phases are possible per model/facing/layer/frame actually encountered.

### Interfaces / Contracts

- `sim/` continues to expose plain data only through `GameEntity.rocking`.
- `render/` must not be imported from `sim/`.
- `app_instances/units.rs` may bridge app state, sim entity data, and render cache access.
- Permanent atlas behavior remains unchanged for non-transition frames.

### Data Flow

```text
rocking_system tick:
  current occupied cell slope -> prev_slope/curr_slope/remaining

unit instance build:
  if transition active:
    transition phase -> transient VXL sprite cache -> SpriteInstance
  else:
    curr_slope -> UnitAtlas -> SpriteInstance
```

Phase mapping should preserve gamemd's timer formula. With a total of 3 frames, render phases should represent `t = 0/3`, `1/3`, `2/3`, then direct current slope once the timer expires. If current Rust decrements the remaining counter before render, the implementation must align the visible phase with the actual tick order or adjust the stored countdown semantics.

### Error Handling

- If transient rendering fails or the cache entry is missing, fall back to the permanent current-slope atlas entry rather than dropping the unit.
- Values outside `0..=16` are clamped to flat using the existing warning path.

### Testing Strategy

- `drive_vxl_slope_change_blends_for_three_frames`: slope transition starts at 3 and expires to current slope after exactly 3 ticks.
- `vxl_slope_transition_phase_uses_previous_then_current`: raster matrix for phase 0 matches previous slope, phase 3/direct matches current slope.
- `vxl_slope_transition_cache_key_distinguishes_from_to_phase`: same type/facing with different phase or slope pair does not collide.
- `unit_instances_use_transition_sprite_when_rocking_slope_active`: active transition avoids direct permanent-atlas slope key.
- Existing `vxl_raster::tests` remain in place to protect the corrected matrix order.

## Architectural Decisions

- Use a transient cache instead of globally pre-baking all slope transitions to avoid atlas blow-up.
- Keep the slope-transition source in existing `RockingState` rather than adding a second locomotor cache.
- Keep interpolation render-only; gameplay/pathfinding remains unchanged.
- Do not change slope signs or slope type mapping.

## Alternatives Considered

### Pre-bake all transition sprites globally

Rejected. It is parity-capable but multiplies atlas size by slope-pair and phase counts for every facing/layer/frame. That is wasteful and likely expensive on large battles.

### Snap to `curr_slope` but use cached state

Rejected for parity. It fixes sampling ownership but not the visible 3-frame blend.

### Blend two already-rendered sprites in screen space

Rejected. Cross-fading previous/current sprites does not reproduce gamemd's matrix interpolation; projected voxel geometry follows a different silhouette.

