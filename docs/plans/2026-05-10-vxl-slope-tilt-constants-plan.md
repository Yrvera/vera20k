# VXL Slope Tilt Constants Implementation Plan

> **For Claude:** Execute this plan task-by-task. Each task is self-contained.

**Goal:** Replace the heuristic edge/corner tilt constants in the voxel renderer with the binary-verified gamemd.exe values, so vehicles on slope_type 1-8 cells lean by the magnitude observed in the original game.

**Architecture:** Pure render-layer change in `src/render/vxl_raster.rs`. Two `const f32` literals + their docstrings + three new tests. The slope-tilt pipeline (cell → atlas key → renderer) is already wired end-to-end; only the magnitudes are wrong.

**Design Doc:** [docs/plans/2026-05-10-vxl-slope-tilt-constants-design.md](docs/plans/2026-05-10-vxl-slope-tilt-constants-design.md)

---

## Grounding Summary

- **Docs:** `ra2-rust-game-docs/VOXEL_SLOPE_TILT_SYSTEM.md` is the authoritative reference (verified GREEN 2026-05-07). It documents the slope-type table, compass directions, matrix construction (`Rz(compass) × Rx(tilt) × Rz(-compass)`), and the two BSS addresses `_DAT_00b44310` (edge) and `_DAT_00b43f08` (corner). It explicitly notes the runtime values for those two doubles were not previously extracted.
- **Ghidra (this session):** Traced the full init chain at `0x00754A50` (edge) and `0x00754A20` (corner) plus the upstream `VXL_Init_CellHeightRatio` (0x007549E0), `VXL_Init_CellDiagonal` (0x00754910), `VXL_Init_CellHalfHeight` (0x007549CC), `VXL_Init_CameraPitch` (0x007549AC), and the literal data words at `0x007E1708/10/28/30/40` and `0x007F6948`. Recovered hidden FMULs the decompiler dropped (the `×0.5` in CellHeightRatio and the rad→BAM scaler at `0x007E8970 = 4096/2π` inside the trig LUT). Chain reduces analytically: `LevelHeight = ftol(tan(π/6) × 256√2 × 0.5) = ftol(128√(2/3)) = 104` (matches the project's known `LevelHeight = 104 leptons`), giving `EDGE = atan(2·104/256√2) = atan(13√2/32) ≈ 0.5214767 rad` and `CORNER = atan(104/256) = atan(13/32) ≈ 0.3858827 rad`.
- **Repo pattern:** [src/render/vxl_raster.rs:46-51](src/render/vxl_raster.rs#L46-L51) and [src/render/vxl_raster.rs:249-267](src/render/vxl_raster.rs#L249-L267) already follow the convention "private `const`s with docstring at the top of the file, consumer in a small dedicated `fn` lower down, tests in the file's own `#[cfg(test)] mod tests`." We mirror that — no new pattern introduced.
- **INI keys:** none. The tilt magnitudes are binary-baked geometry, not INI-driven.
- **Stale-check:** `git log -- src/render/vxl_raster.rs` shows only `Initial commit` and `apply cargo fmt to entire codebase` — design doc's line numbers and current-state claims are accurate.
- **Still unknown:** the tilt sign convention (whether glam's right-handed `Mat4::from_rotation_x` matches gamemd's tilt direction). The geometry test in Task 3 resolves this; if it fails, the existing constants masked the issue with a too-shallow tilt.

## Key Technical Decisions

- **`f32` not `f64` for the constants** — Confidence: high. **Source:** repo pattern; existing constants in the file are `f32` (line 46/51), and `compute_slope_rotation` returns `Mat4` (which is `f32`-based in glam). Using `f64` would force casts at every consumer.
- **Underscore-grouped numeric literals** (`0.521_540_3`) — Confidence: high. **Source:** Edition 2024 / clippy convention for readability of numeric constants longer than 4 digits.
- **Geometry test asserts `south.z > north.z` for slope_type=4** — Confidence: high. **Source:** `VOXEL_SLOPE_TILT_SYSTEM.md §Slope Type Values` — slope_type 4 is named "South" and means "south corners raised". The test mirrors that semantic and would fail loudly if either the magnitude OR the sign of the tilt drifts.
- **Tripwire tests recompute the formula in-test** — Confidence: high. **Source:** Standard pattern for protecting magic numbers against accidental edits; the test re-derives `atan(2·104/256√2)` so a future edit that moves the constant without recomputing the formula gets caught.
- **No update to ra2-rust-game-docs/VOXEL_SLOPE_TILT_SYSTEM.md** — Confidence: medium. **Source:** that doc is the verified-GREEN research archive. The two literals' values are now known and the doc still says "Their initialization source is outside the decompiled C file range" which is now stale. Plan ends with an optional task to update the doc; user can choose.

## Open Questions

### Resolved During Planning

- *Should the change touch `vxl_compute.rs` (GPU path) too?* No — `unit_atlas.rs` is the actual caller of `prepare_limb_data` (lines 636, 648, 668 for body/turret/barrel), and it feeds the resulting `LimbRenderData.combined` matrix to both the CPU rasterizer (`render_vxl` in `vxl_raster.rs`) and the GPU compute path (`vxl_compute.rs`). `vxl_compute.rs` itself does not call `prepare_limb_data` or `compute_slope_rotation` — it just consumes the pre-baked transform. Both paths therefore see the corrected constants automatically. Verified by grep.
- *Does the unit atlas need explicit invalidation?* No — there's no on-disk cache for tilted unit sprites; the atlas rebuilds on app start.
- *Will any `slope_type` consumer break from a magnitude change?* No — the consumers see a `Mat4`, not the angle. They don't introspect tilt magnitude.

### Deferred to Implementation

- *Sign of the tilt under glam's right-handed `from_rotation_x`* — resolved by the geometry test in Task 3. If it fails, the fix is to negate `tilt_rad` in `compute_slope_rotation`. We don't pre-emptively negate — that would mask a wrong handedness if both sign AND magnitude were changed at once.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | `src/render/vxl_raster.rs` (lines 43-51, plus tests module ~line 624) | Update two const values + their docstrings; add three tests |

No other files touched.

## Interface Changes

None. `EDGE_TILT_RAD` and `CORNER_TILT_RAD` are private (`const`, no `pub`). `compute_slope_rotation`'s signature is unchanged. The atlas key schema (`UnitSpriteKey`) and `prepare_limb_data` signature are unchanged.

## Sim Checklist

Not applicable — no sim/ files touched. Render-only change. World hash unaffected.

## Risk Areas

- **Sign/handedness regression** (Task 3 geometry test catches it). If glam's right-handed convention disagrees with gamemd's, the test fails and the fix is `Mat4::from_rotation_x(-tilt_rad)` in `compute_slope_rotation`. **Pre-existing risk** — not introduced by this change, just exposed by stronger constants.
- **Atlas cached entries from a previous run** — none on disk, but if the user has the app running with a built atlas, they need to restart to see the new tilt. Document in the commit message.
- **Visual regression on currently-correct-looking units** — the heuristic constants happened to produce a tilt that "looked plausible." Stronger correct values may look more dramatic than expected on a casual eyeball test. Verify against gamemd, not against memory.

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|--------|------|----------------|--------------|
| Task 1 | `EDGE_TILT_RAD = 0.5214767 rad` (was `0.3876`) | Every ground vehicle on slope_type 1-4 (west/north/east/south edge ramps) leans by this amount. Visible every match on every ramp. ~35% steeper than current | Tripwire test in Task 4 + geometry test in Task 3 + visual check vs gamemd |
| Task 1 | `CORNER_TILT_RAD = 0.3858827 rad` (was `0.2810`) | Every ground vehicle on slope_type 5-8 (corner ramps) leans by this. Visible every match on every corner ramp. ~37% steeper | Tripwire test in Task 4 + geometry test in Task 3 + visual check vs gamemd |
| Task 3 | Tilt direction matches gamemd's compass convention | A wrong sign would make units lean the *opposite* way (e.g., south-facing slope tips north side up instead of south side). Visible immediately on any ramp once magnitudes are corrected | `test_slope_4_south_corner_rises` — slope_type=4 must produce `south.z > north.z` |
| Task 5 | Visual smoke check against gamemd.exe | Catches whole-pipeline regressions the unit tests can't see (e.g., facing × slope multiplication order, atlas re-render artifacts, fixed-point projection sign) | Manual: load a map with visible ramps, drop a Rhino, compare to gamemd screenshot of same setup |

---

## Tasks

### Task 1: Replace the two tilt constants and update docstrings

**Why:** This is the core fix. The two `const f32` literals at lines 46 and 51 are heuristic guesses; replace them with the gamemd-derived values and replace the docstrings with the actual derivation chain.

**Files:**
- Modify: `src/render/vxl_raster.rs` lines 43-51

**Pattern:** Existing convention in the same file — private `const f32` with docstring above. No new pattern.

**Step 1: Replace the edge constant block**

Find this block in [src/render/vxl_raster.rs:43-46](src/render/vxl_raster.rs#L43-L46):

```rust
/// Edge ramp tilt angle (slope types 1-4): `atan(tan(30°) / sqrt(2))`.
/// Derived from RA2's isometric geometry — one full cell edge raised by one height
/// level. `VXL_Init_EdgeTiltAngle` computes `atan(2h / diag)`.
const EDGE_TILT_RAD: f32 = 0.3876;
```

Replace with:

```rust
/// Edge ramp tilt angle (slope types 1-4): `atan(2 × LevelHeight / cellDiagonal)`.
///
/// Reduces analytically to `atan(13√2 / 32) ≈ 0.5214767 rad ≈ 29.88°`, where
/// `LevelHeight = 104 leptons` (the canonical RA2 vertical step) and
/// `cellDiagonal = 256√2 leptons`. The factor of 2 reflects the rise across
/// the full diagonal of a one-level edge ramp (two adjacent corners raised).
const EDGE_TILT_RAD: f32 = 0.521_540_3;
```

**Step 2: Replace the corner constant block**

Find this block in [src/render/vxl_raster.rs:48-51](src/render/vxl_raster.rs#L48-L51):

```rust
/// Corner ramp tilt angle (slope types 5-8): `atan(tan(30°) / 2)`.
/// One corner raised by one height level across the cell diagonal.
/// `VXL_Init_CornerTiltAngle` computes `atan(h / 256)`.
const CORNER_TILT_RAD: f32 = 0.2810;
```

Replace with:

```rust
/// Corner ramp tilt angle (slope types 5-8): `atan(LevelHeight / cellSide)`.
///
/// Reduces analytically to `atan(13 / 32) ≈ 0.3858827 rad ≈ 22.10°`, where
/// `LevelHeight = 104 leptons` and `cellSide = 256 leptons`. One corner of
/// the cell is raised half a level; the run is one cell side (not the
/// diagonal) — that's what distinguishes corner from edge ramps.
const CORNER_TILT_RAD: f32 = 0.385_866_0;
```

**Step 3: Verify file compiles**

Run: `cargo check --lib`
Expected: PASS (no compile errors).

**Step 4: Commit**

Run:
```
git add src/render/vxl_raster.rs
git commit -m "render/vxl: correct slope tilt constants to gamemd-verified values

EDGE_TILT_RAD: 0.3876 → 0.521540 (atan(13√2/32))
CORNER_TILT_RAD: 0.2810 → 0.385866 (atan(13/32))

Prior values were heuristic guesses (atan(tan(30°)/√2) and atan(tan(30°)/2));
the gamemd init chain at 0x00754A50/0x00754A20 reduces analytically around
LevelHeight=104 leptons. Vehicles on slope_type 1-8 cells now lean ~35-37%
more, matching the original game."
```

---

### Task 2: Add the magnitude tripwire tests

**Why:** Protect the new constants against silent corruption — if someone edits the literal but not the derivation comment (or vice versa), the test fails. Pure-formula tests, no game state needed.

**Files:**
- Modify: `src/render/vxl_raster.rs` (extend the existing `#[cfg(test)] mod tests` near the bottom of the file, ~line 624)

**Pattern:** Existing tests in the same module already use this style — `#[test] fn test_x()` with `assert!` and `assert_eq!`.

**Step 1: Add the two magnitude tests**

In `src/render/vxl_raster.rs`, locate the existing `mod tests` block (currently starting around line 624 with `use super::*;`). Add the following two tests after `test_axis_order_negative_depth` (the last existing test):

```rust
#[test]
fn test_edge_tilt_magnitude_matches_gamemd_formula() {
    // Tripwire: catches accidental edits to EDGE_TILT_RAD that don't recompute
    // the gamemd derivation chain. atan(2 × 104 / (256 × √2)) is the value
    // VXL_Init_EdgeTiltAngle (0x00754A50) stores at DAT_00B44310 in gamemd.exe.
    let expected: f32 = (2.0_f32 * 104.0 / (256.0 * 2.0_f32.sqrt())).atan();
    assert!(
        (EDGE_TILT_RAD - expected).abs() < 1e-5,
        "EDGE_TILT_RAD={} drifted from gamemd formula {}",
        EDGE_TILT_RAD,
        expected
    );
}

#[test]
fn test_corner_tilt_magnitude_matches_gamemd_formula() {
    // Tripwire: catches accidental edits to CORNER_TILT_RAD. atan(104 / 256)
    // is what VXL_Init_CornerTiltAngle (0x00754A20) stores at DAT_00B43F08.
    let expected: f32 = (104.0_f32 / 256.0).atan();
    assert!(
        (CORNER_TILT_RAD - expected).abs() < 1e-5,
        "CORNER_TILT_RAD={} drifted from gamemd formula {}",
        CORNER_TILT_RAD,
        expected
    );
}
```

**Step 2: Run the new tests**

Run: `cargo test --lib -p vera20k vxl_raster::tests::test_edge_tilt_magnitude_matches_gamemd_formula vxl_raster::tests::test_corner_tilt_magnitude_matches_gamemd_formula`

(Package name `vera20k` from `Cargo.toml`. Single-package workspace — the `-p` flag is optional; this simpler form also works:)

Run: `cargo test --lib tilt_magnitude`
Expected: 2 passed.

**Step 3: Commit**

Run:
```
git add src/render/vxl_raster.rs
git commit -m "render/vxl: tripwire tests for slope tilt constants

Re-derive atan(2·104/256√2) and atan(104/256) inside the test to catch
silent drift from the gamemd-verified values."
```

---

### Task 3: Add the geometry / direction test

**Why:** The tripwires in Task 2 only verify magnitude. A wrong tilt *sign* (e.g., glam right-handed disagreeing with gamemd's convention) would still pass them. This test confirms slope_type=4 ("South" = south corners raised) actually raises the south side of the model — catches sign and handedness bugs in one shot.

**Files:**
- Modify: `src/render/vxl_raster.rs` (extend same `mod tests` block from Task 2)

**Pattern:** Same — `#[test] fn` in the existing tests module.

**Step 1: Add the direction test**

After the two tests added in Task 2, append:

```rust
#[test]
fn test_slope_4_south_high_corner_rises() {
    // Slope type 4 = "South" per VOXEL_SLOPE_TILT_SYSTEM.md: south corners
    // are raised. After applying compute_slope_rotation(4) to a model-space
    // unit vector pointing in +Y ("north" in model space) and -Y ("south"),
    // the south point must end up higher in world Z than the north point.
    //
    // This test catches sign/handedness regressions — a wrong-direction tilt
    // (e.g., from glam's right-handed Rx disagreeing with gamemd's convention)
    // would flip the inequality and fail loudly.
    let slope_mat: Mat4 = compute_slope_rotation(4);

    let north: Vec3 = slope_mat.transform_point3(Vec3::Y);
    let south: Vec3 = slope_mat.transform_point3(-Vec3::Y);

    assert!(
        south.z > north.z,
        "Expected south corner raised for slope_type=4; got north.z={}, south.z={}",
        north.z,
        south.z
    );
}
```

**Step 2: Run the test**

Run: `cargo test --lib slope_4_south_high_corner_rises`
Expected: PASS.

**If it FAILS:** the tilt is in the wrong direction. The fix is to change [src/render/vxl_raster.rs:264-266](src/render/vxl_raster.rs#L264-L266):

```rust
Mat4::from_rotation_z(compass_rad)
    * Mat4::from_rotation_x(tilt_rad)
    * Mat4::from_rotation_z(-compass_rad)
```

to:

```rust
Mat4::from_rotation_z(compass_rad)
    * Mat4::from_rotation_x(-tilt_rad)
    * Mat4::from_rotation_z(-compass_rad)
```

(Negating `tilt_rad` inverts the X-rotation direction.) Re-run the test, confirm PASS, and add a second commit explaining the sign correction. **Do NOT silently flip the sign without the test failing first** — that would mask whether the original code was right by accident or wrong by accident.

**Step 3: Commit (only if test passed without modification)**

Run:
```
git add src/render/vxl_raster.rs
git commit -m "render/vxl: geometry test for slope tilt direction

Asserts slope_type=4 (\"South\" = south corners raised) actually raises
the south side of the model — catches sign/handedness bugs that magnitude
tripwires miss."
```

If the test failed and you applied the sign fix in the alternative path above, instead commit:
```
git add src/render/vxl_raster.rs
git commit -m "render/vxl: invert tilt sign for glam right-handed convention

The geometry test (slope_type=4 should raise south side) failed with
positive tilt_rad — glam's Mat4::from_rotation_x rotates Y toward Z,
which inverts gamemd's tilt direction. Negate tilt_rad in
compute_slope_rotation to match."
```

---

### Task 4: Run the full test suite as a regression check

**Why:** Confirm the constants change and new tests didn't break any existing voxel rendering tests (`test_render_produces_nonempty_sprite`, `test_facing_changes_output`, etc.) or any other code that depends on `vxl_raster`.

**Files:** none modified.

**Step 1: Run the renderer test set**

Run: `cargo test --lib render::vxl_raster`
Expected: all tests pass, including the existing four (`test_render_produces_nonempty_sprite`, `test_empty_model_returns_transparent`, `test_facing_changes_output`, `test_point_plot_fills_pixels`, `test_voxel_grid_packing`, `test_axis_order_positive_depth`, `test_axis_order_negative_depth`) PLUS the three new ones.

**Step 2: Run the unit-atlas tests** (atlas keys depend on slope_type)

Run: `cargo test --lib render::unit_atlas`
Expected: PASS — atlas key schema is unchanged, just the rendered pixel content differs.

**Step 3: Run the full library test suite**

Run: `cargo test --lib`
Expected: no new failures relative to the baseline before Task 1. Anything that fails here is unexpected and needs investigation before continuing.

**Step 4: No commit** — this task is verification only.

---

### Task 5: Visual smoke check against gamemd.exe

**Why:** Unit tests verify magnitude and direction in isolation. They don't catch whole-pipeline regressions (atlas re-render artifacts, facing × slope multiplication subtleties, fixed-point projection sign on tilted models). One eyeball-check on a real unit on a real ramp closes that gap.

**Files:** none modified.

**Step 1: Pick a map with clearly visible ramps**

Any standard skirmish map with cliff-side ramps — e.g., a Tour of Egypt or country roads map. The map needs at least one tile of slope_type 1, 2, 3, or 4 (full edge ramps, the most common).

**Step 2: Run the engine, drop a Rhino on each visible ramp**

Run: `cargo run --release` (or whatever launch command the project uses).

For each of slope types 1-8 you can find on the map, place or move a Rhino tank onto the slope cell. Note that:
- Slope_type 1 (West) — west corners high → Rhino should tip toward the east
- Slope_type 2 (North) — north corners high → Rhino should tip toward the south
- Slope_type 3 (East) — east corners high → Rhino should tip toward the west
- Slope_type 4 (South) — south corners high → Rhino should tip toward the north
- Slope_type 5-8 (corners) — only one corner raised, gentler tilt

**Step 3: Compare to gamemd.exe**

Launch original gamemd.exe with the same (or equivalent) map and unit setup. Compare side-by-side. The Rust render should match gamemd in tilt direction and approximate magnitude; corner ramps (5-8) should look noticeably gentler than edge ramps (1-4).

**Step 4: Note any disparities**

If the Rust tilt looks right: done. Note "visual parity confirmed against gamemd on slope_type X-Y" in the commit message of Task 6 (or just verbally to the user).

If there's a visible disparity in either direction or magnitude: stop and investigate. Possible causes:
- Sign error caught by Task 3 but a different sign convention exists between body and world rotation (re-check the `combined = rotate_to_world * slope_mat * section_transform` order in `prepare_limb_data:328`).
- Atlas caching an old tilted variant from before the constant change (rebuild and retest).
- An unrelated rendering bug exposed by the new magnitude.

**Step 5: No commit** — verification only.

---

### Task 6 (optional): Update VOXEL_SLOPE_TILT_SYSTEM.md to reflect the resolved constants

**Why:** The research doc currently says "Their initialization source is outside the decompiled C file range. They likely represent the isometric camera tilt angle scaled for the terrain slope steepness." That's now stale — the values and chain are pinned. Updating the doc means future sessions don't waste time re-discovering this.

**Files:**
- Modify: `docs/research/VOXEL_SLOPE_TILT_SYSTEM.md` (the "Tilt Angle Constants" section, lines 126-137)

**Pattern:** Existing doc style — markdown with tables, addresses, and explicit confidence/sourcing.

**Step 1: Replace the "Tilt Angle Constants" section**

Find the section starting at "## Tilt Angle Constants" (around line 126) and ending before "## Direction Encoding in param_3" (around line 139). Replace its body with:

```markdown
## Tilt Angle Constants

| Address | Value | Used For | Initialized By |
|---|---|---|---|
| `_DAT_00b44310` | `0.5214767 rad` (≈29.88°) | Full-edge ramp tilt (types 1-4) | `VXL_Init_EdgeTiltAngle` (0x00754A50) |
| `_DAT_00b43f08` | `0.3858827 rad` (≈22.10°) | Corner ramp tilt (types 5-8) | `VXL_Init_CornerTiltAngle` (0x00754A20) |

Both reduce to clean closed forms around `LevelHeight = 104 leptons`:

- **Edge:** `atan(2 × LevelHeight / cellDiagonal) = atan(2 × 104 / 256√2) = atan(13√2/32)`
- **Corner:** `atan(LevelHeight / cellSide) = atan(104 / 256) = atan(13/32)`

### Init Chain (verified 2026-05-10)

```
DAT_00B43F00 (CameraPitch)    = (π/180) × 60 = π/3              [VXL_Init_CameraPitch    @ 0x007549AC]
DAT_00B43ED8 (CellHalfHeight) = (π/180) × 90 = π/2              [VXL_Init_CellHalfHeight @ 0x007549CC]
DAT_00B43EF8 (CellDiagonal)   = sqrt_approx(2 × pow(256, 2))    [VXL_Init_CellDiagonal   @ 0x00754910]
                              = 256√2 ≈ 362.04 leptons

# VXL_Init_CellHeightRatio @ 0x007549E0:
#   The decompile drops a hidden `× 0.5` (FMUL [0x007E1738]) that the asm shows.
#   The "Sin_Lookup_Table4096" (0x004CAD50) is actually a tan LUT and has its
#   own hidden `× 4096/(2π)` scaler at 0x007E8970 to convert radians→BAM index.
DAT_00B45578 (LevelHeight) = ftol(tan(π/2 − π/3) × 256√2 × 0.5)
                           = ftol(tan(π/6) × 128√2)
                           = ftol(128√(2/3))
                           = ftol(104.532...) = 104

DAT_00B44310 = atan(2 × 104 / 256√2) ≈ 0.5214767 rad
DAT_00B43F08 = atan(104 × 1/256)     ≈ 0.3858827 rad
```

The `0x007F6948` constant used by `Init_CameraPitch` and `Init_CellHalfHeight`
is `π/180` (degrees-to-radians); the literals at `0x007E1708/10/28/30` are
`2.0`, `256.0`, `60.0`, `90.0`; `0x007E1740` is `1/256`. **Confidence:** verified
from binary in the 2026-05-10 session.

The labels `CameraPitch` and `CellHalfHeight` are Ghidra-applied guesses for
the 60° and 90° literals — they don't correspond to RA2's actual iso camera
angle (~26.57°). The difference (90° − 60° = 30°) is what the tan LUT
samples to drive the LevelHeight calculation.
```

**Step 2: Commit the doc update**

Run (from the docs repo):
```
cd <local>/Documents/ra2-rust-game-docs
git add VOXEL_SLOPE_TILT_SYSTEM.md
git commit -m "vxl slope tilt: pin the two tilt constants from binary trace

Resolved DAT_00B44310 = 0.5214767 rad (= atan(13√2/32)) and
DAT_00B43F08 = 0.3858827 rad (= atan(13/32)) by tracing the full init
chain through VXL_Init_CellHeightRatio (0x007549E0). LevelHeight = 104
leptons drops out cleanly, matching the spatial primitives doc."
```

---

## Sources & References

- **Design doc:** [docs/plans/2026-05-10-vxl-slope-tilt-constants-design.md](docs/plans/2026-05-10-vxl-slope-tilt-constants-design.md)
- **Ghidra report:** `docs/research/VOXEL_SLOPE_TILT_SYSTEM.md` — slope-type table, compass directions, matrix construction, two-path body/turret split
- **gamemd.exe addresses (this session, all verified):**
  - `0x00754A50` `VXL_Init_EdgeTiltAngle` — writes `DAT_00B44310`
  - `0x00754A20` `VXL_Init_CornerTiltAngle` — writes `DAT_00B43F08`
  - `0x007549E0` `VXL_Init_CellHeightRatio` — computes `DAT_00B45578` (LevelHeight=104)
  - `0x00754910` `VXL_Init_CellDiagonal` — computes `DAT_00B43EF8` (256√2)
  - `0x007549CC` `VXL_Init_CellHalfHeight` — π/2
  - `0x007549AC` `VXL_Init_CameraPitch` — π/3
  - `0x004CAD50` (mislabeled "Sin_Lookup_Table4096", actually tan LUT) — has hidden `×4096/(2π)` scaler at `0x007E8970`
  - `0x004CADE0` `atan` (LUT-based)
  - `0x004CAC40` `Sqrt_Approx`
  - Literal data: `0x007E1708 = 2.0`, `0x007E1710 = 256.0`, `0x007E1728 = 60.0`, `0x007E1730 = 90.0`, `0x007E1738 = 0.5`, `0x007E1740 = 1/256`, `0x007F6948 = π/180`, `0x007E8970 = 4096/2π`
- **INI keys:** none — this is binary-baked geometry
- **Related code:** [src/render/vxl_raster.rs](src/render/vxl_raster.rs), [src/render/vxl_compute.rs](src/render/vxl_compute.rs), [src/render/unit_atlas.rs](src/render/unit_atlas.rs), [src/app_instances/units.rs](src/app_instances/units.rs), [src/map/resolved_terrain.rs](src/map/resolved_terrain.rs)
- **Spatial primitives anchor:** `LevelHeight = 104 leptons` matches the constant documented in `SPATIAL_PRIMITIVES_LAYER_GHIDRA_REPORT.md`
