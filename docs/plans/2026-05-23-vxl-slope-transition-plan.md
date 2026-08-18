# VXL Slope Transition — Implementation Plan

> Execute this plan task-by-task. Do not broaden into unrelated voxel rendering cleanup.

**Goal:** Implement standard YR's 3-frame VXL vehicle slope transition so voxel units blend from previous terrain slope to current terrain slope instead of snapping between discrete slope atlas sprites.

**Design Doc:** [docs/plans/2026-05-23-vxl-slope-transition-design.md](2026-05-23-vxl-slope-transition-design.md)

---

## Parity Confidence

**High confidence on target behavior.** The relevant gamemd behavior is verified by the May 23 targeted reports:

- `VXL_SLOPE_CELL_SAMPLING_GHIDRA_REPORT.md`: Drive locomotor samples the current occupied cell slope, caches previous/current slope, and starts a 3-frame transition on slope change.
- `VXL_DRAW_MATRIX_ORDER_GHIDRA_REPORT.md`: simple VXL transform order is `camera_view * slope_matrix * facing_rotation * section_or_hva_matrix`.
- `VXL_SLOPE_MATRIX_SIGN_GHIDRA_REPORT.md`: the slope matrix sign is already correct; do not negate `tilt_rad`.

**Implementation confidence depends on two details:**

1. **Interpolation math.** Before coding the blend helper, re-check `VXL_InterpolatedFacing` and the slope matrix/table path it consumes. The implementation must reproduce gamemd's matrix/quaternion path closely enough that phase `0` equals the previous slope, phase `3/3` equals the current slope, and intermediate phases match gamemd's rendered orientation. Do not substitute generic matrix lerp or a convenience `Quat::slerp` unless that is first proven equivalent to the binary path for all populated slope IDs.
2. **Phase timing.** Rust's existing `RockingState.transition_ticks_remaining` is decremented inside the sim tick. Before wiring render, Task 2 must verify whether app rendering observes `3,2,1` or `2,1,0` after the transition-start tick. The implementation must map that observed value back to gamemd's `t = (total - remaining) / total`.

**What this plan does not prove by itself:** pixel-identical screenshots. After implementation, run a visual trace/screenshot with a Chrono Miner or tracked vehicle entering and leaving a ramp. Passing tests plus verified matrix math should make it parity-correct, but the final player-visible judgment should still include that visual pass.

## Grounding Summary

- Current Rust already tracks slope transition state in `RockingState`, but render ignores it.
- Current permanent `UnitAtlas` pre-renders `slope_type = 0..=16` for ground VXL units.
- Current `app_instances/units.rs` samples `ResolvedTerrainCell.slope_type` directly and emits one discrete atlas key.
- The previous matrix-order fix in `vxl_raster.rs` should remain unchanged: slope is left of body facing and right of camera/view.

## Key Technical Decisions

- **Use `RockingState` as the cached slope source.** It already stores previous/current slope and a 3-tick countdown. No new sim-to-render state is needed.
- **Do not globally pre-bake transition sprites.** Pre-baking all `from_slope * to_slope * phase * facing * layer * frame` combinations would explode atlas size.
- **Generate transient sprites on demand.** Cache only transition sprites actually requested by visible units.
- **Do not blend two rendered sprites.** Gamemd blends the slope matrix, not the final 2D sprite. Cross-fading silhouettes is visibly different.
- **Keep gameplay deterministic untouched.** This is render-only consumption of existing sim state.

## Open Questions

### Resolved During Planning

- **Is interpolation active in stock YR?** Yes. `DriveLocomotionClass::Process` writes the 3-frame transition total on occupied-cell slope changes.
- **Is the fix a sign flip?** No. Matrix sign matches gamemd; the earlier visual issue was matrix order, and transition smoothing is a separate visual behavior.
- **Should destination/next cell slope be used?** No. Binary samples current occupied cell.

### Deferred To Implementation

- **Exact Rust phase mapping:** determine whether render sees the newly started transition before or after one decrement, then encode the correct `t` formula.
- **Exact cache page sizing/eviction:** start with a session-scoped cache and only add eviction if memory use becomes observable during visual/battle tests.

## File Map

| Action | Path | Responsibility |
|---|---|---|
| Modify | [src/render/vxl_raster.rs](../../src/render/vxl_raster.rs) | Add blended slope render params and matrix tests |
| Modify | [src/render/unit_atlas.rs](../../src/render/unit_atlas.rs) | Expose/reuse single-sprite VXL rendering helpers for transient cache |
| Add | `src/render/unit_slope_transition_cache.rs` | Session-scoped transient atlas pages for blended VXL slope sprites |
| Modify | [src/app_instances/units.rs](../../src/app_instances/units.rs) | Select atlas vs transient transition sprite per entity/layer |
| Modify | [src/app.rs](../../src/app.rs) | Own `VxlSlopeTransitionCache` beside `unit_atlas` |
| Modify | [src/app_render/build_instances.rs](../../src/app_render/build_instances.rs) | Pass mutable cache context and return permanent/transient unit instance buckets |
| Modify | [src/app_render/merge_passes.rs](../../src/app_render/merge_passes.rs) | Merge permanent unit atlas group plus transient unit atlas-page groups in Y-sort order |
| Modify | [src/app_render/draw_passes.rs](../../src/app_render/draw_passes.rs) | Pass transient atlas-page textures to merged bridge/object passes |
| Modify | [src/sim/rocking/rocking_system.rs](../../src/sim/rocking/rocking_system.rs) | Only if phase semantics need comment/test adjustment; no render dependency |
| Add/Modify tests | render/app tests near affected modules | Lock phase, cache key, and matrix behavior |

## Interface Changes

Add a render-only slope blend representation:

```rust
pub struct VxlSlopeBlend {
    pub from_slope: u8,
    pub to_slope: u8,
    pub phase_num: u8,
    pub phase_den: u8, // expected 3
}
```

`VxlRenderParams` should gain an optional `slope_blend`. When `None`, existing `slope_type` behavior remains unchanged.

Add a transition cache key:

```rust
TransitionUnitSpriteKey {
    type_id,
    facing,
    layer,
    frame,
    from_slope,
    to_slope,
    phase_num,
}
```

The cache value must provide the same placement data app rendering needs from `UnitSpriteEntry`: texture/UV, pixel size, offsets, and shader path metadata.

Cache integration must be atlas-page based, not one GPU texture per transition sprite. `VxlSlopeTransitionCache` should own one or more R8Uint `BatchTexture` pages plus a key-to-entry map. Transition instances should be bucketed by page, then drawn through the same voxel sprite shader and `PaletteSet` as `UnitAtlas` instances. The permanent `UnitAtlas` remains unchanged.

## Sim Checklist

- [x] No new sim dependency on render/ui/sidebar/audio/net.
- [x] No gameplay math changes.
- [x] No new deterministic state required if existing `RockingState` remains authoritative.
- [x] World hash already includes `prev_slope`, `curr_slope`, and `transition_ticks_remaining` in `src/sim/world/world_hash.rs`.
- [ ] Confirm transition countdown phase is deterministic and independent of render frame rate at the app render boundary, not only in the pure rocking helper.

## Risk Areas

- **Cache blow-up:** avoid global pre-bake; keep on-demand/session-scoped cache.
- **GPU texture churn:** avoid allocating a unique texture every draw if many units share the same transition key.
- **Off-by-one phase:** most likely parity bug. Tests must lock first visible transition frame and expiry frame.
- **Layer coherence:** composite/body/turret/barrel must all use the same transition phase for a unit.
- **Fallback correctness:** if transient rendering fails, unit must still render via current-slope permanent atlas.

## Parity-Critical Items

| Task # | Item | Source | Verification |
|---|---|---|---|
| 1 | Current occupied cell slope is source | `VXL_SLOPE_CELL_SAMPLING_GHIDRA_REPORT.md` | Unit/app slope source test |
| 2 | Duration exactly 3 frames | `CDTimerClass__Start(3)` at `0x004B052D` | Rocking transition phase test |
| 3 | Matrix interpolation, not sprite cross-fade | `Draw_Matrix` calls `VXL_InterpolatedFacing` | Ghidra spot-check plus raster matrix tests |
| 4 | Direct current slope after expiry | `VXL_GetFacingMatrix` direct path | Expiry test |
| 5 | Slope left of facing | `VXL_DRAW_MATRIX_ORDER_GHIDRA_REPORT.md` | Existing/new `vxl_raster` tests |

---

## Tasks

### Task 1: Add slope-blend render params and matrix helpers

**Why:** `vxl_raster.rs` needs to render a matrix between two slope matrices without changing the permanent discrete slope path.

**Files:**
- Modify: [src/render/vxl_raster.rs](../../src/render/vxl_raster.rs)

**Steps:**

1. Add `VxlSlopeBlend` and `Option<VxlSlopeBlend>` to `VxlRenderParams`.
2. Re-check `VXL_InterpolatedFacing` before implementing the helper. Record the verified formula/table behavior in the task notes or a short doc update.
3. Add `compute_slope_blend_rotation(from, to, phase_num, phase_den)` beside `compute_slope_rotation`.
4. Implement the helper from the verified binary behavior. If the binary path uses a slope quaternion/table, mirror that table-derived orientation instead of interpolating arbitrary matrices.
5. Clamp `from/to > 16` to identity using the same defensive policy as discrete slope rendering.
6. For `slope_blend = Some`, use the blended matrix instead of `compute_slope_rotation(params.slope_type)`.
7. Keep the existing composition order: `camera_view * slope_mat * body_facing * section_transform`.

**Tests:**

- `vxl_slope_blend_phase_zero_matches_previous_slope`
- `vxl_slope_blend_phase_full_matches_current_slope`
- `vxl_slope_blend_preserves_camera_slope_facing_order`
- `vxl_slope_blend_midphase_matches_verified_interpolated_facing_case`

**Acceptance:**

- Existing `vxl_raster::tests` still pass.
- Flat/no-blend output path is unchanged.
- The helper is not a placeholder/generic interpolation; it has a cited binary-equivalence note.

### Task 2: Verify and lock Rust transition phase semantics

**Why:** The only serious parity uncertainty is whether render observes countdown values before or after the first decrement.

**Files:**
- Modify tests in [src/sim/rocking/rocking_tests.rs](../../src/sim/rocking/rocking_tests.rs)
- Possibly modify comments in [src/sim/rocking/rocking_system.rs](../../src/sim/rocking/rocking_system.rs)

**Steps:**

1. Add/adjust a test that starts from `curr_slope = 0`, calls `update_slope_transition(..., 4)`, and records countdown values after subsequent ticks.
2. Determine the render-facing phase function:
   - If render sees `3` on the first transition frame: `phase_num = 0`.
   - If render sees `2` on the first transition frame: `phase_num = 1`.
3. Inspect `Simulation::advance_tick` and `app_render::build_world_instances` ordering to verify the value the renderer actually observes after a movement tick starts a transition.
4. Encode a small pure helper in app/render code, not in sim, for `remaining -> phase_num`.
5. Add a narrow app/render-boundary test or harness that builds a unit instance immediately after a tick that changes `curr_slope`, proving the first visible transition frame uses the intended phase.

**Tests:**

- `drive_vxl_slope_transition_phase_counts_three_visible_frames`
- `drive_vxl_slope_transition_expires_to_current_slope`
- `unit_render_phase_after_transition_start_matches_gamemd_mapping`

**Acceptance:**

- The implementation has an explicit documented phase mapping to gamemd `t = (total - remaining) / total`.
- The mapping is proven at the app render boundary, not just inside `rocking_system.rs`.

### Task 3: Add transient transition sprite cache

**Why:** Transition sprites should not repack the permanent atlas and should not be regenerated for every visible unit every frame.

**Files:**
- Add: `src/render/unit_slope_transition_cache.rs`
- Modify: [src/app.rs](../../src/app.rs)
- Modify: [src/app_render/build_instances.rs](../../src/app_render/build_instances.rs)
- Modify: [src/app_render/merge_passes.rs](../../src/app_render/merge_passes.rs)
- Modify: [src/app_render/draw_passes.rs](../../src/app_render/draw_passes.rs)
- Modify [src/render/unit_atlas.rs](../../src/render/unit_atlas.rs) if helper extraction is needed.

**Steps:**

1. Define `TransitionUnitSpriteKey`.
2. Define `TransitionUnitSpriteEntry` compatible with `SpriteInstance` placement.
3. Add `VxlSlopeTransitionCache` owned by `AppState` beside `unit_atlas`.
4. Add a helper to render one transition sprite with `VxlRenderParams { slope_blend: Some(...) }`.
5. Pack transition sprites into append-only transient atlas pages owned by the cache. Each page must be an R8Uint `BatchTexture` compatible with the existing voxel sprite shader.
6. Bucket generated transition `SpriteInstance`s by transient page so `merge_passes` can Y-merge permanent `UnitAtlas` units and transient units without one draw call per sprite.
7. Recreate or append only the affected transient page when a new key is inserted; never repack the permanent `UnitAtlas`.
8. Keep cache session-scoped initially; only add eviction if memory use becomes observable.

**Tests:**

- `vxl_slope_transition_cache_key_distinguishes_from_to_phase`
- `vxl_slope_transition_cache_reuses_existing_entry`
- `vxl_slope_transition_cache_packs_multiple_keys_on_one_page`
- `merged_object_pass_accepts_permanent_and_transient_voxel_groups`

**Acceptance:**

- Same transition key is generated once and reused.
- Different `from/to/phase` keys do not collide.
- Normal battles do not allocate one GPU texture per transition sprite.
- Draw order remains Y-sorted across permanent VXL, transient VXL, SHP, and wall groups.

### Task 4: Wire app unit instances to choose transient sprites

**Why:** `app_instances/units.rs` currently always chooses a discrete permanent atlas slope.

**Files:**
- Modify: [src/app_instances/units.rs](../../src/app_instances/units.rs)

**Steps:**

1. Replace direct terrain slope lookup with a helper that returns a render slope state:
   - `Stable(slope_type)`
   - `Transition { from_slope, to_slope, phase_num }`
2. For aircraft, always return `Stable(0)`.
3. For ground voxel entities with `rocking.transition_ticks_remaining > 0`, use `rocking.prev_slope` and `rocking.curr_slope`.
4. For stable ground entities, prefer `rocking.curr_slope` when `rocking` exists; fall back to current terrain cell lookup for entities without rocking.
5. If `rocking.curr_slope == 0` but the current terrain cell is a valid nonzero slope and the entity has not yet received a rocking tick, seed the render-only stable slope from terrain for that first frame. Do not mutate sim state from render.
6. Apply the same render slope state to composite/body/turret/barrel layers.
7. Preserve existing slope `>=17` warning/clamp behavior.
8. If transition cache lookup/render fails, fall back to stable `curr_slope` atlas entry.

**Tests:**

- `unit_instances_use_transition_sprite_when_rocking_slope_active`
- `unit_instances_use_curr_slope_after_transition_expires`
- `unit_instances_spawned_on_slope_render_slope_before_first_rocking_tick`
- `unit_instances_aircraft_ignore_slope_transition`
- `unit_instances_slope_ge_17_clamps_to_flat`
- `unit_instances_turret_layers_share_one_transition_phase`

**Acceptance:**

- Visible VXL ground vehicles use transition sprites only while transition is active.
- Normal non-transition rendering still uses permanent atlas entries.
- A vehicle first rendered on a ramp does not flash flat for one frame.

### Task 5: Keep permanent atlas unchanged

**Why:** Stable rendering should remain fast and memory-bounded.

**Files:**
- Modify only comments/tests in [src/render/unit_atlas.rs](../../src/render/unit_atlas.rs), unless helper extraction is required.

**Steps:**

1. Confirm `needed_unit_sprite_keys` still pre-bakes only `0..=16`.
2. Do not add slope-pair/phase variants to permanent atlas.
3. Update stale comments that still say "9 slope variants" if present.

**Tests:**

- Existing unit atlas tests.
- Add `unit_atlas_ground_vehicles_prebake_17_slope_variants` if a helper exists to test cheaply.

**Acceptance:**

- Permanent atlas size does not multiply by transition phases.

### Task 6: Run focused verification

**Commands:**

```powershell
cargo fmt --check
cargo test vxl_raster::tests
cargo test rocking::rocking_tests
cargo test unit_atlas
cargo test unit_slope_transition_cache
cargo test app_instances::units
cargo test app_render::merge_passes
```

Run broader tests only if touched files or failures indicate it:

```powershell
cargo test
```

**Acceptance:**

- Focused tests pass.
- Any unrelated warnings are noted, not fixed.

### Task 7: Visual verification

**Why:** This is a player-visible rendering bug.

**Steps:**

1. Run a local scenario/map where a Chrono Miner or tracked vehicle drives down/up a known ramp.
2. Capture before/after or inspect live movement.
3. Confirm:
   - vehicle lean direction is world-ramp oriented,
   - tilt does not pop instantly at the cell boundary,
   - parked vehicle on ramp settles to the final slope,
   - aircraft remain unaffected.

**Acceptance:**

- If visual still looks wrong, stop and reassess. Do not layer more changes without identifying whether the issue is phase timing, cache sprite generation, matrix interpolation, or terrain slope byte mapping.

---

## Stop Conditions

- A test shows Rust's `RockingState` countdown cannot reproduce gamemd's 3 visible frames without changing sim tick semantics.
- Implementing transient sprites requires repacking the entire permanent atlas every frame.
- The only feasible implementation becomes a sprite cross-fade. That is not parity; stop and redesign.
- Visual verification is worse after the change.
