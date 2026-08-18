# VXL Slope Rust Handoff Report

Report path: `docs/research/VXL_SLOPE_RUST_HANDOFF_REPORT.md`

Scope: read-only Rust scan for Chrono Miner / voxel vehicle uphill-looking tilt on slopes. No Ghidra used. No Rust edited.

## Rust Facts

1. `src/render/vxl_raster.rs:266-293` owns the current slope matrix: `compute_slope_rotation(slope_type)` maps 1-16 to `(compass_rad, tilt_rad)` and applies `Rz(compass) * Rx(tilt_rad) * Rz(-compass)` with positive `tilt_rad`.
2. `src/render/vxl_raster.rs:354-355` composes slope before section transform and after facing/camera setup: `combined = rotate_to_world * slope_mat * section_transform`.
3. `src/render/unit_atlas.rs:208-223`, `src/render/unit_atlas.rs:327-340`, and `src/render/unit_atlas.rs:422-434` pre-bake ground vehicle atlas entries for slope variants `0..=16`; `src/render/unit_atlas.rs:595-599` passes each key's slope to `VxlRenderParams`.
4. `src/app_instances/units.rs:108-130` samples `state.resolved_terrain.cell(pos.rx,pos.ry).slope_type` for non-aircraft, clamps raw values `>=17` to `0`, and passes that single current-cell slope into composite/body/turret/barrel keys at `src/app_instances/units.rs:221-230` and `src/app_instances/units.rs:370-391`.
5. `src/sim/rocking/rocking_system.rs:165-171` and `src/sim/components.rs:717-722` maintain `prev_slope`, `curr_slope`, and a 3-tick transition counter, but `rg` found no render/app read of those fields; only sim hashing/tests reference them (`src/sim/world/world_hash.rs:547-549`, `src/sim/rocking/rocking_tests.rs:182-221`).

## Implementation Handoff

- Verified current Rust behavior: slope direction/sign is isolated in `compute_slope_rotation`; sibling swarm claim now reports gamemd positive Rx matches current Rust for slope 4/2 -> Rust delta: do not negate `tilt_rad`; instead lock the verified sign with clearer tests and investigate sampling/matrix-order if the symptom remains -> affected surface: `src/render/vxl_raster.rs` -> acceptance scenario: synthetic asymmetric VXL on slope 4 tilts with +Y raised and slope 2 tilts oppositely -> proposed test name: `test_slope_4_verified_retail_direction_sign` -> risk: changing sign would regress all pre-baked vehicle slope sprites.
- Verified current Rust behavior: atlas already has 0..=16 variants and app keys include the sampled slope -> Rust delta: do not expand atlas range or rebuild behavior for this bug; add a key-collection regression to lock current coverage -> affected surface: `src/render/unit_atlas_tests.rs` or nearby unit atlas tests -> acceptance scenario: one ground voxel entity collects every slope `0..=16`, aircraft collect only `0` -> proposed test name: `test_ground_vehicle_key_collection_includes_all_populated_slope_variants` -> risk: test fixture setup may need small helpers for `EntityStore`, rules, and interner.
- Verified current Rust behavior: app instance selection uses the current terrain cell, not `RockingState` transition fields -> Rust delta: if the player-visible bug is a one-cell snap/sampling issue rather than sign, extract the slope-selection clamp into a pure helper and test current-cell vs aircraft vs `>=17` behavior before touching render math -> affected surface: `src/app_instances/units.rs` -> acceptance scenario: CMIN/MTNK on terrain slope 4 uses atlas key slope 4, aircraft on slope 4 uses key 0, slope 17 logs/clamps to 0 -> proposed test name: `test_unit_instance_slope_key_uses_current_cell_for_ground_units` -> risk: `build_unit_instances` is app-heavy, so helper extraction should stay local and avoid sim dependencies.

## Focused Tests To Add

- `src/render/vxl_raster.rs`: replace or extend `test_slope_4_geometry_locks_current_direction`; assert the verified current sign for slope 4 and the opposite sign for slope 2.
- `src/render/vxl_raster.rs` or `src/render/voxel_parity_tests.rs`: render a tiny asymmetric/tall VXL with `slope_type=0`, `2`, and `4`; compare projected top/bottom pixel displacement so a sign or matrix-order regression fails without retail assets.
- `src/render/unit_atlas_tests.rs`: lock ground vehicle slope variant coverage `0..=16` and aircraft `0`, because the atlas side is already broad enough and should not be blamed for missing slope 9-16 variants.
- `src/app_instances/units.rs` tests if a pure helper is extracted: ground CMIN/MTNK current-cell slope, aircraft flat override, and `>=17` flat fallback.
- Do not run a full app screenshot test until the matrix sign is verified; a screenshot will reveal the symptom but will not localize sign vs sampling vs sprite-key fallback.

## Do Not Do

- Do not enable render-time `RockingState` slope SLERP as the default fix. `VXL_INTERPOLATED_FACING_AND_SLOPE_TRANSITION_GHIDRA_REPORT.md:261-265` says standard YR uses direct lookup with transition total `0`; current Rust also does not read those fields in app/render.
- Do not expand atlas baking beyond `0..=16` for this bug. `src/render/unit_atlas.rs:212-213`, `src/render/unit_atlas.rs:329-330`, and `src/render/unit_atlas.rs:423-424` already cover the populated matrix range; `src/app_instances/units.rs:121-127` clamps `>=17` to flat.
- Do not treat `test_slope_4_geometry_locks_current_direction` as retail proof. Its own comments at `src/render/vxl_raster.rs:824-828` say the +Y/-Y world correspondence still needs visual confirmation and may require negating `tilt_rad`.
- Do not negate `tilt_rad` for this bug unless a newer direct contradiction appears; `.swarm-claims.md` records slot-1 `VXL_Slope_Matrix_Sign_Retry` as done with positive Rx matching current Rust.
- Do not move slope selection into `sim/` or make `sim/` depend on render/UI. The current app layer can read terrain and choose atlas keys without violating the repo layering rule.
- Do not fix a sign bug by changing terrain/pathfinding `slope_type` semantics; pathfinding already uses slope bytes for traversal gates in separate tests, while this player-visible issue is in voxel render orientation.

## Remaining Uncertainty

- No Ghidra was used in this slot; sign confidence comes only from sibling slot-1's claims-file summary, not from independent verification here.
- The player symptom "uphill-looking" is now less likely to be raw `tilt_rad` sign and more likely matrix composition/order, slope-cell sampling timing, or a mismatch between model/world axis interpretation and sprite projection.
- The docs say stock YR does not use slope interpolation by default, but Rust currently maintains transition state in sim; whether that state should be removed, ignored, or retained for future non-default use needs owner decision.

## Stale Code Comment Wording

- `src/render/unit_atlas.rs:188-189`: replace "pre-render all 9 slope variants (0=flat, 1-8=ramps) upfront" with "pre-render all 17 slope variants (0=flat, 1-16 populated slope matrices) upfront".
- `src/sim/rocking/rocking_system.rs:163-164`: replace "The render side reads `prev_slope`/`curr_slope`/`transition_ticks_remaining` to SLERP between the two slope matrices." with "These fields are retained for the slope-transition model; the current stock-parity render path does not read them and uses direct current-cell slope lookup."
- `src/sim/components.rs:699-700` and `src/sim/components.rs:721`: replace the unconditional "3-tick quaternion-SLERP slope transition" wording with "tracked slope transition state; stock-parity rendering currently ignores it unless a future verified transition path is deliberately wired."

Status: COMPLETE
