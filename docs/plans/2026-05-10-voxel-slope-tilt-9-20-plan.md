# Voxel Slope Tilt Rendering — Slopes 9-20 Implementation Plan

> **For Claude:** Execute this plan task-by-task. Each task is self-contained.

**Goal:** Extend the VXL slope-tilt renderer to cover gamemd.exe's full populated
slope-matrix table (slopes 9-16) and apply a defensive identity clamp for the
unpopulated entries (slopes 17-20), so ground vehicles tilt correctly on every
ramp variant a YR map can emit.

**Architecture:** Render-only change. Sim continues to read `cell.slope_type: u8`
from the parsed TMP `ramp_type` byte unchanged. The render hand-off in
`app_instances/units.rs` widens the slope-byte clamp from `<= 8` to `<= 16`; the
unit atlas widens its pre-render slope range from `0..=8` to `0..=16` at three
call sites; `compute_slope_rotation` in `vxl_raster.rs` gains 8 new match arms
covering slopes 9-16 with their exact (compass, tilt) pairs from gamemd.exe's
runtime-populated `DAT_00b45188` table.

**Design Doc:** [docs/plans/2026-05-10-voxel-slope-tilt-9-20-design.md](docs/plans/2026-05-10-voxel-slope-tilt-9-20-design.md)

---

## Grounding Summary

**R1 — ra2-rust-game-docs/.** The primary source is
`ra2-rust-game-docs/VOXEL_SLOPE_TILT_SYSTEM.md` and specifically its
"Slope Matrix Table — Full Entry List" addendum dated 2026-05-10, which
verified all 16 populated slope entries (1-16) directly from
`VXL_MasterLighting_Init` (`0x00754CB0`) and `VXL_GetFacingMatrix`
(`0x007559B0`) in gamemd.exe. The addendum also confirms slopes 17-20
are BSS-zero (no matrix populated) via `inspect_memory_content @
0x00B454B8`. The older parent doc `VXL_DRAW_MATRIX_GHIDRA_REPORT.md`
covers the same system at lower detail and is superseded by the addendum
for slopes ≥ 9. `LAT_GROUPS_AND_SLOPE_FIXUP_GHIDRA_REPORT.md` documents
TMP-byte → cell-slope-index propagation but covers the LAT fixup pass,
not the vehicle tilt pipeline; cited here only for completeness.

**R2 — Ghidra MCP.** No new live verification needed. The 2026-05-10
addendum captured every binary fact this plan depends on: per-entry
compass/tilt for slopes 1-16, BSS-zero confirmation for 17-20, the
no-bounds-clamp lookup at `VXL_GetFacingMatrix`, and the
`Rz(c) · Rx(t) · Rz(-c)` composition. **Confidence:** verified
binary-direct in the prior session.

**R3 — Repo patterns.** The plan mirrors the existing slopes-1-8
implementation landed on `dev` in commits `169d42a`, `0f2fa4a`, `af411d5`
(slope tilt constants + tripwire tests + slope-4 geometry lockdown).
The match arm in `src/render/vxl_raster.rs:255-273`, the consumer clamp
in `src/app_instances/units.rs:81-87`, and the three atlas pre-render
sites in `src/render/unit_atlas.rs:210-211, 333-334, 432-433` are all
extended in-place with the same code shape — no new abstractions, no new
files.

**R4 — INI keys.** None. Slope tilt is driven by the per-tile TMP
`ramp_type` byte (file format, not INI), and the tilt magnitudes are
hardcoded gamemd constants (`EDGE_TILT_RAD`, `CORNER_TILT_RAD`) already
defined in `vxl_raster.rs:46,52` from the slopes-1-8 work. No INI parser
changes required.

**Unknowns after grounding.** Whether any standard YR map TMP actually
emits `slope_type ∈ [17, 20]` is empirically unknown (Q6 deferred per
brainstorm decision). The throttled `warn!` log in Task 4 surfaces this
at runtime if it occurs.

## Key Technical Decisions

- **Slopes 9-16: extend the existing match arm in `compute_slope_rotation`**
  with 8 new arms following the slopes-1-8 pattern. **Confidence:** high.
  - **Source:** `ra2-rust-game-docs/VOXEL_SLOPE_TILT_SYSTEM.md` "Per-entry
    breakdown" rows 9-16; repo pattern `src/render/vxl_raster.rs:255-273`.
- **Slopes 17-20: clamp to identity at the consumer (`units.rs`).** The
  atlas never sees keys for 17-20; `compute_slope_rotation` retains its
  defensive `_ => Mat4::IDENTITY` arm as in-depth defense. **Confidence:** high.
  - **Source:** brainstorm decision (option b); `VOXEL_SLOPE_TILT_SYSTEM.md`
    "DAT_00b454B8 ... NOT POPULATED" + GHIDRA `inspect_memory_content @
    0x00B454B8`.
- **Atlas pre-render range widens to `0..=16`** at all three call sites.
  The runtime atlas-miss → `slope_type=0` fallback at
  `units.rs:281-298` already handles any cache-miss path. **Confidence:** high.
  - **Source:** repo pattern (existing 0..=8 logic at three call sites);
    brainstorm decision Q4.
- **Throttled `warn!` on first `slope_type >= 17`** uses an
  `AtomicBool` one-shot, branch-free fast path after the first fire.
  **Confidence:** high.
  - **Source:** standard Rust `std::sync::atomic` pattern; brainstorm
    decision Q6 ("defer + add warn-log on 17-20 contact").

No low-confidence decisions — every binding fact is verified-from-binary
in the addendum.

## Open Questions

### Resolved During Planning

- **Are any commits on dev since the brainstorm (2026-05-10) likely to
  invalidate the design's assumed file state?** No. `git log -10 --
  src/render/vxl_raster.rs src/app_instances/units.rs
  src/render/unit_atlas.rs` shows the most recent touch is `5e2594b`
  (`barrel_facing.current(binary_frame)`) on `units.rs`, which does not
  intersect the slope hand-off region at lines 81-87. Working tree clean.
- **Do tests need to verify exact float matrix values for slopes 9-16, or
  is structural verification (alias equality + non-identity for new
  combos) sufficient?** Structural verification is sufficient. Aliases
  (9-12 == 5-8) are byte-identical per the addendum, so direct matrix
  equality is the strongest possible test. New combos (13-16) get a
  not-equal-to-aliased-corner test that catches the most likely
  regression (accidental swap of EDGE/CORNER constant). The
  `EDGE_TILT_RAD` / `CORNER_TILT_RAD` magnitudes themselves already have
  formula-tripwire tests at `vxl_raster.rs:799-826`.

### Deferred to Implementation

- **Will the post-change atlas overflow the
  `max_texture_dimension_2d` limit on a typical map?** Cannot answer
  until measured on a real map. The existing retry-grow logic
  (`unit_atlas.rs:1094-1132`) emits a `warn!` if it does. Tracked as a
  follow-up: if the warning fires, multi-page packing for the unit
  atlas becomes the next task.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | `src/render/vxl_raster.rs:66-68` | Update `VxlRenderParams::slope_type` doc comment range |
| Modify | `src/render/vxl_raster.rs:255-273` | Extend `compute_slope_rotation` match with slopes 9-16 |
| Modify | `src/render/vxl_raster.rs` (test module) | Add slope 9 alias, slope 13 distinct-from-5, and slopes-17-20 identity tests |
| Modify | `src/render/unit_atlas.rs:65-67` | Update `UnitSpriteKey::slope_type` doc comment range |
| Modify | `src/render/unit_atlas.rs:210-211, 333-334, 432-433` | Widen pre-render slope range from `0..=8` to `0..=16` |
| Modify | `src/app_instances/units.rs:76-87` | Widen consumer clamp from `<= 8` to `<= 16`; emit one-shot `warn!` on `slope_type >= 17` |

## Interface Changes

- `VxlRenderParams::slope_type: u8` — public field. Documented range
  widens from `0..=8` to `0..=16`. Type unchanged. **Consumers:**
  `vxl_raster::compute_slope_rotation` (this PR), `unit_atlas::render_unit_sprite`.
- `UnitSpriteKey::slope_type: u8` — public field. Documented range
  widens from `0..=8` to `0..=16`. Type unchanged. `Hash`/`Eq` derives
  unaffected. **Consumers:** `unit_atlas` build/lookup, `app_instances/units.rs`
  atlas key construction.

No trait or function signatures change. No struct layouts change.

## Sim Checklist

This plan does not touch `sim/`. Slope-tilt rendering is render-only —
no tick-order changes, no determinism impact, no state-hash impact. The
sim/render layering invariant is preserved (sim writes
`cell.slope_type` once at map load; render reads it per frame).

## Risk Areas

From the design doc's Impact Analysis:

- **Atlas size growth (~89% in slope variants per ground vehicle).** The
  existing single-texture retry-grow logic may double atlas width to
  fit. If it overflows `max_texture_dimension_2d`, the warning at
  `unit_atlas.rs:1121-1130` fires and atlas height is clamped — sprites
  beyond the limit silently drop. Mitigation: monitor the build log
  after the change; if the warning fires, add multi-page packing as a
  follow-up.
- **Slope ≥ 17 in real map data.** Unknown frequency. Identity-clamp
  + warn-log surfaces this at runtime without breaking rendering.
- **Per-frame branch on the new clamp.** The widened `<= 16` clamp adds
  no measurable cost vs. `<= 8`. The one-shot `warn!` is gated by an
  `AtomicBool::load(Relaxed)` fast path — branch-free after first fire.

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|--------|------|----------------|--------------|
| Task 1 | Slopes 9-12 must produce **byte-identical** matrices to slopes 5-8 (CORNER tilt, NW/NE/SE/SW). | Player-visible: a vehicle on a slope-9 cell should tilt the exact same way as on a slope-5 cell. Any drift becomes a per-cell visual stutter as the unit walks across cells of different `slope_type` that gamemd renders identically. | Test: `slope_mat_9 == slope_mat_5` (and 10==6, 11==7, 12==8). Source: `VOXEL_SLOPE_TILT_SYSTEM.md` "Per-entry breakdown" rows 9-12 — "byte-identical to entry 5/6/7/8". |
| Task 1 | Slopes 13-16 use the **EDGE tilt magnitude** (0.521_476_7 rad) at corner directions (NW/NE/SE/SW). NEW combo not present in 1-8. | Player-visible: steeper diagonal-corner ramps lean more than mid-corner ramps. Confusing EDGE vs. CORNER on these cells produces a visibly weaker tilt. | Test: `slope_mat_13 != slope_mat_5` (same compass, different tilt magnitude). Source: `VOXEL_SLOPE_TILT_SYSTEM.md` rows 13-16. |
| Task 1 | Composition order is `Rz(compass) · Rx(tilt) · Rz(-compass)` (same as slopes 1-8). No negated angles, no half-angles. | Player-visible: wrong composition tilts the unit toward the wrong direction (e.g., toward downhill instead of along the ramp). | Pattern reuse: same `Mat4::from_rotation_z(c) * Mat4::from_rotation_x(t) * Mat4::from_rotation_z(-c)` line covers all 16 slopes. Source: addendum "Matrix builder" §. |
| Task 2 | Atlas pre-renders all 17 slope variants (`0..=16`) so unit visuals never depend on a runtime atlas rebuild. | Player-visible: an atlas miss falls back to slope 0 (flat). If pre-rendering misses 9-16, every vehicle on a steep ramp renders flat instead of tilted on first contact. | Build-log line: `"Unit atlas: ... N total needed"` count rises by ~89% per ground vehicle vs. pre-change baseline. |
| Task 3 | Consumer clamp `slope_type > 16 → 0` (not `> 8 → 0`). | Player-visible: bytes 9-16 must reach the atlas key, not be folded to flat. Otherwise vehicles on slopes 9-16 render flat (regression). | Code review of the clamp condition; visual smoke check on a sandbox map with slope-9 / slope-13 cells. |
| Task 3 | Slopes 17-20 produce a **flat (untilted)** unit, not an invisible one. | Player-visible: gamemd renders these as invisible (BSS-zero matrix); we deliberately diverge to a more robust failure mode (CLAUDE.md "internals modernized, outputs preserved"). | Defensive `_ => Mat4::IDENTITY` arm in `compute_slope_rotation` + consumer clamp combine to guarantee. Verified by Task 1's slopes-17-20 identity test. |

---

## Tasks

### Task 1: Extend `compute_slope_rotation` for slopes 9-16

**Why:** Render core is the lowest-risk, most isolated change. Once
`compute_slope_rotation` produces correct matrices for 9-16, the rest
of the plan is plumbing. Tests in this task lock the parity-critical
behavior in place so subsequent tasks cannot silently break it.

**Files:**
- Modify: `src/render/vxl_raster.rs:66-68` (doc comment on `VxlRenderParams::slope_type`)
- Modify: `src/render/vxl_raster.rs:249-273` (function doc + match arm)
- Modify: `src/render/vxl_raster.rs` test module (add three tests)

**Pattern:** mirrors existing slopes-1-8 match arms (`vxl_raster.rs:256-269`)
and the lockdown-test style at `vxl_raster.rs:828-860`
(`test_slope_4_geometry_locks_current_direction`).

**Step 1: Update the `slope_type` doc comment on `VxlRenderParams`**

Replace lines 66-68 of `src/render/vxl_raster.rs`:

```rust
    /// Terrain slope type (0–8). 0 = flat, 1-4 = edge ramps, 5-8 = corner ramps.
    /// The VXL model is tilted to match the terrain slope before camera projection.
    pub slope_type: u8,
```

with:

```rust
    /// Terrain slope type (0–16). 0 = flat, 1-4 = edge ramps (full-edge tilt),
    /// 5-8 = corner ramps (corner tilt at NW/NE/SE/SW), 9-12 = corner tilt at
    /// NW/NE/SE/SW (byte-identical aliases of 5-8 in gamemd.exe), 13-16 = edge
    /// tilt at NW/NE/SE/SW. Slopes 17-20 are unpopulated in gamemd (BSS-zero
    /// matrix); the consumer clamps them to 0 before this field is set.
    pub slope_type: u8,
```

**Step 2: Update the function doc + match arm of `compute_slope_rotation`**

Replace lines 249-273 of `src/render/vxl_raster.rs`:

```rust
/// Compute the slope rotation matrix for a given terrain slope type (0–8).
///
/// Formula: `slope_matrix = Rz(compass) * Rx(tilt) * Rz(-compass)`
/// where compass is the slope direction angle and tilt is the pitch amount.
///
/// Returns `Mat4::IDENTITY` for slope_type 0 (flat) or unknown types (9+).
fn compute_slope_rotation(slope_type: u8) -> Mat4 {
    let (compass_rad, tilt_rad): (f32, f32) = match slope_type {
        0 => return Mat4::IDENTITY,
        // Edge ramps (two adjacent corners raised one height level).
        1 => (4.7124, EDGE_TILT_RAD),               // West,  270°
        2 => (std::f32::consts::PI, EDGE_TILT_RAD), // North, 180°
        3 => (std::f32::consts::FRAC_PI_2, EDGE_TILT_RAD), // East,  90°
        4 => (0.0, EDGE_TILT_RAD),                  // South, 0°
        // Corner ramps (one corner raised one height level).
        5 => (3.9270, CORNER_TILT_RAD), // NW, 225°
        6 => (2.3562, CORNER_TILT_RAD), // NE, 135°
        7 => (0.7854, CORNER_TILT_RAD), // SE, 45°
        8 => (5.4978, CORNER_TILT_RAD), // SW, 315°
        _ => return Mat4::IDENTITY,     // slopes 9-20: treat as flat for now
    };
    Mat4::from_rotation_z(compass_rad)
        * Mat4::from_rotation_x(tilt_rad)
        * Mat4::from_rotation_z(-compass_rad)
}
```

with:

```rust
/// Compute the slope rotation matrix for a given terrain slope type (0–16).
///
/// Formula: `slope_matrix = Rz(compass) * Rx(tilt) * Rz(-compass)`
/// where compass is the slope direction angle and tilt is the pitch amount.
///
/// Slopes 9-12 produce the same matrices as 5-8 (corner tilt at NW/NE/SE/SW).
/// Slopes 13-16 reuse the corner directions but with the steeper edge tilt
/// magnitude — a combination not present in slopes 1-8.
///
/// Returns `Mat4::IDENTITY` for slope_type 0 (flat) and as a defensive
/// fallback for any value ≥ 17 that bypasses the consumer-side clamp.
fn compute_slope_rotation(slope_type: u8) -> Mat4 {
    let (compass_rad, tilt_rad): (f32, f32) = match slope_type {
        0 => return Mat4::IDENTITY,
        // Edge ramps (two adjacent corners raised one height level).
        1 => (4.7124, EDGE_TILT_RAD),                      // West,  270°
        2 => (std::f32::consts::PI, EDGE_TILT_RAD),        // North, 180°
        3 => (std::f32::consts::FRAC_PI_2, EDGE_TILT_RAD), // East,  90°
        4 => (0.0, EDGE_TILT_RAD),                         // South, 0°
        // Corner ramps (one corner raised one height level).
        5 => (3.9270, CORNER_TILT_RAD), // NW, 225°
        6 => (2.3562, CORNER_TILT_RAD), // NE, 135°
        7 => (0.7854, CORNER_TILT_RAD), // SE, 45°
        8 => (5.4978, CORNER_TILT_RAD), // SW, 315°
        // Diagonal-corner CORNER tilt (byte-identical aliases of 5-8).
        9 => (3.9270, CORNER_TILT_RAD),  // NW, 225°
        10 => (2.3562, CORNER_TILT_RAD), // NE, 135°
        11 => (0.7854, CORNER_TILT_RAD), // SE, 45°
        12 => (5.4978, CORNER_TILT_RAD), // SW, 315°
        // Diagonal-corner EDGE tilt (steeper variant of 9-12).
        13 => (3.9270, EDGE_TILT_RAD), // NW, 225°
        14 => (2.3562, EDGE_TILT_RAD), // NE, 135°
        15 => (0.7854, EDGE_TILT_RAD), // SE, 45°
        16 => (5.4978, EDGE_TILT_RAD), // SW, 315°
        _ => return Mat4::IDENTITY,    // slopes 17-20: defensive identity clamp
    };
    Mat4::from_rotation_z(compass_rad)
        * Mat4::from_rotation_x(tilt_rad)
        * Mat4::from_rotation_z(-compass_rad)
}
```

**Step 3: Add three regression tests**

Append to the `mod tests` block in `src/render/vxl_raster.rs` (after
`test_slope_4_geometry_locks_current_direction`, before the closing `}`
of the test module at line 861):

```rust
    #[test]
    fn test_slopes_9_to_12_alias_corner_ramps_5_to_8() {
        // gamemd's VXL_MasterLighting_Init populates slope-table entries 9-12
        // with the same compass+tilt arguments as 5-8 (CORNER tilt at
        // NW/NE/SE/SW). The matrices are byte-identical at runtime.
        // A regression that swapped CORNER for EDGE on 9-12 would tilt these
        // cells more steeply than gamemd does — a player-visible drift.
        for (extended, base) in [(9, 5), (10, 6), (11, 7), (12, 8)] {
            let ext_mat: Mat4 = compute_slope_rotation(extended);
            let base_mat: Mat4 = compute_slope_rotation(base);
            assert_eq!(
                ext_mat, base_mat,
                "slope_type={} should produce the same matrix as slope_type={}",
                extended, base
            );
        }
    }

    #[test]
    fn test_slopes_13_to_16_use_edge_tilt_at_corner_directions() {
        // Slopes 13-16 reuse the corner compass directions (NW/NE/SE/SW from
        // 5-8) but with the steeper EDGE tilt magnitude — a combination not
        // present in slopes 1-8. The matrix must therefore differ from the
        // CORNER-tilt variant at the same compass.
        for (steep, corner) in [(13, 5), (14, 6), (15, 7), (16, 8)] {
            let steep_mat: Mat4 = compute_slope_rotation(steep);
            let corner_mat: Mat4 = compute_slope_rotation(corner);
            assert_ne!(
                steep_mat, corner_mat,
                "slope_type={} (EDGE tilt) must not equal slope_type={} (CORNER tilt)",
                steep, corner
            );
            // Sanity: also not identity.
            assert_ne!(
                steep_mat,
                Mat4::IDENTITY,
                "slope_type={} should produce a tilt, not identity",
                steep
            );
        }
    }

    #[test]
    fn test_slopes_17_to_20_return_identity() {
        // gamemd has no matrix populated for slopes 17-20 (BSS-zero region
        // at DAT_00b454B8). We deliberately diverge from gamemd's invisible-
        // unit failure mode and clamp these to identity (flat) at the
        // renderer. The consumer clamp in app_instances/units.rs is the
        // primary boundary; this defensive arm catches any value that
        // bypasses it.
        for slope in 17..=20u8 {
            assert_eq!(
                compute_slope_rotation(slope),
                Mat4::IDENTITY,
                "slope_type={} must clamp to identity",
                slope
            );
        }
    }
```

**Step 4: Verify**

Run:

```
cargo test --lib --package vera20k render::vxl_raster -- --nocapture
```

Expected: all existing slope tests still PASS, three new tests PASS.

Then run a clippy pass on the file:

```
cargo clippy --lib --package vera20k -- -D warnings
```

Expected: no new warnings introduced.

**Step 5: Commit**

```
git add src/render/vxl_raster.rs
git commit -m "render/vxl: extend compute_slope_rotation for slopes 9-16

Slopes 9-12 alias the CORNER-tilt corner ramps 5-8 (byte-identical
matrices in gamemd.exe). Slopes 13-16 use the steeper EDGE tilt
magnitude at the same NW/NE/SE/SW corner directions — a combination
not present in 1-8. Slopes 17-20 retain a defensive identity clamp;
gamemd renders these with an unpopulated BSS-zero matrix (unit
becomes invisible), which we deliberately diverge from to a flat
(untilted) render.

Tests lock the alias equality (9==5, 10==6, 11==7, 12==8), the
distinctness of EDGE-at-corner from CORNER-at-corner (13!=5 etc.),
and the identity clamp for 17-20."
```

---

### Task 2: Widen unit-atlas pre-render slope range to 0..=16

**Why:** With the renderer producing correct matrices for slopes 9-16,
the atlas must pre-render those variants so vehicles on those cells
render tilted from the moment of first contact instead of falling back
to flat via the runtime atlas-miss path.

**Files:**
- Modify: `src/render/unit_atlas.rs:65-67` (doc comment on `UnitSpriteKey::slope_type`)
- Modify: `src/render/unit_atlas.rs:207-211` (call site #1: `collect_needed_unit_keys`)
- Modify: `src/render/unit_atlas.rs:333-334` (call site #2: `build_unit_atlas` step 1)
- Modify: `src/render/unit_atlas.rs:432-433` (call site #3: `build_unit_atlas` step 1c, UnloadingClass referents)

**Pattern:** all three call sites already use the same
`std::ops::RangeInclusive<u8>` shape. We only widen the upper bound.

**Step 1: Update the `slope_type` doc comment on `UnitSpriteKey`**

Replace lines 65-67 of `src/render/unit_atlas.rs`:

```rust
    /// Terrain slope type (0–8). 0 = flat, 1-4 = edge ramps, 5-8 = corner ramps.
    /// Different slopes produce distinct pre-rendered sprites with tilted models.
    pub slope_type: u8,
```

with:

```rust
    /// Terrain slope type (0–16). 0 = flat, 1-4 = edge ramps, 5-8 = corner
    /// ramps, 9-12 = corner tilt at NW/NE/SE/SW (alias of 5-8 in gamemd.exe),
    /// 13-16 = edge tilt at NW/NE/SE/SW. The consumer in app_instances/units.rs
    /// clamps any value ≥ 17 to 0 before constructing this key. Different
    /// slopes produce distinct pre-rendered sprites with tilted models.
    pub slope_type: u8,
```

**Step 2: Widen pre-render range at call site #1 (`collect_needed_unit_keys`)**

Replace lines 208-211 of `src/render/unit_atlas.rs`:

```rust
            // Ground vehicles: generate all 9 slope variants (0-8) so no
            // atlas rebuild is needed when driving onto ramps.
            // Aircraft: only slope_type=0 (flat).
            let slope_range: std::ops::RangeInclusive<u8> =
                if is_ground_vehicle { 0..=8 } else { 0..=0 };
```

with:

```rust
            // Ground vehicles: pre-render all 17 slope variants (0-16) so no
            // atlas rebuild is needed when driving onto any populated ramp
            // (gamemd has no matrices for slopes 17-20; the consumer in
            // app_instances/units.rs clamps those to 0). Aircraft never tilt.
            let slope_range: std::ops::RangeInclusive<u8> =
                if is_ground_vehicle { 0..=16 } else { 0..=0 };
```

**Step 3: Widen pre-render range at call site #2 (`build_unit_atlas` step 1)**

Replace lines 333-334 of `src/render/unit_atlas.rs`:

```rust
            let slope_range: std::ops::RangeInclusive<u8> =
                if is_ground_vehicle { 0..=8 } else { 0..=0 };
```

with:

```rust
            // Ground vehicles: 17 slope variants (0-16) covering every
            // populated entry in gamemd's slope-matrix table.
            let slope_range: std::ops::RangeInclusive<u8> =
                if is_ground_vehicle { 0..=16 } else { 0..=0 };
```

**Step 4: Widen pre-render range at call site #3 (UnloadingClass referents)**

Replace lines 432-433 of `src/render/unit_atlas.rs`:

```rust
            let slope_range: std::ops::RangeInclusive<u8> =
                if is_ground_vehicle { 0..=8 } else { 0..=0 };
```

with:

```rust
            // Same slope range as the parent type (0-16 for ground vehicles).
            let slope_range: std::ops::RangeInclusive<u8> =
                if is_ground_vehicle { 0..=16 } else { 0..=0 };
```

**Step 5: Verify**

Run:

```
cargo check --lib --package vera20k
```

Expected: clean build.

Run:

```
cargo test --lib --package vera20k render::unit_atlas
```

Expected: existing unit_atlas tests PASS. (No new tests added in this
task — the slope-range widening is data-only and is exercised by Task 4's
end-to-end run.)

**Step 6: Commit**

```
git add src/render/unit_atlas.rs
git commit -m "render/atlas: pre-render slope variants 0..=16 for ground vehicles

Widens the unit atlas pre-render range from 0..=8 (the slopes-1-8
work) to 0..=16 (the full populated range in gamemd.exe). Aircraft
remain at 0..=0 (no terrain tilt).

Updated at all three pre-render call sites: collect_needed_unit_keys,
build_unit_atlas step 1, and UnloadingClass referent seeding. Atlas
size grows ~89% in slope variants per ground vehicle; existing
single-texture retry-grow logic handles the increase, and the runtime
atlas-miss → slope_type=0 fallback at app_instances/units.rs:281-298
remains the safety net."
```

---

### Task 3: Widen consumer clamp + add throttled warn-log on slope ≥ 17

**Why:** With the atlas now pre-rendering 0..=16, the consumer must let
those bytes pass through to the atlas key instead of folding them to 0.
The throttled `warn!` log surfaces any real map that emits slopes
≥ 17, providing data for the deferred TMP empirical scan without
blocking the renderer fix.

**Files:**
- Modify: `src/app_instances/units.rs:81-87` (clamp + warn-log)

**Pattern:** the existing clamp is exactly the boundary we widen. The
one-shot warn-log uses `std::sync::atomic::AtomicBool::compare_exchange`
— this is the standard Rust pattern for "fire once across the process."

**Step 1: Add the warn-log helper function**

Insert at the top of `src/app_instances/units.rs`, after the existing
`use` block (i.e., after line 23, before `pub(crate) fn build_unit_instances`):

```rust
use std::sync::atomic::{AtomicBool, Ordering};

/// One-shot tripwire: fires the first time a `slope_type >= 17` byte is
/// observed at the render hand-off. Subsequent observations are silent
/// (single Relaxed load on the fast path, branch-prediction friendly).
///
/// Slopes 17-20 are unpopulated in gamemd's runtime slope-matrix table
/// (BSS-zero at DAT_00b454B8 per VOXEL_SLOPE_TILT_SYSTEM.md). The
/// existence of such bytes in shipping TMP data is empirically unknown;
/// this log surfaces them at runtime so the deferred TMP scan can be
/// scheduled if it ever fires.
static WARNED_SLOPE_GE_17: AtomicBool = AtomicBool::new(false);

fn warn_unexpected_slope_once(slope: u8, rx: u16, ry: u16) {
    if WARNED_SLOPE_GE_17.load(Ordering::Relaxed) {
        return;
    }
    if WARNED_SLOPE_GE_17
        .compare_exchange(false, true, Ordering::Relaxed, Ordering::Relaxed)
        .is_ok()
    {
        log::warn!(
            "slope_type {} encountered at cell ({}, {}); gamemd has no \
             matrix populated for slopes 17-20 — rendering flat. This \
             is the first observation of this slope range in the \
             current process; subsequent observations are silent.",
            slope,
            rx,
            ry,
        );
    }
}
```

**Step 2: Widen the consumer clamp and wire the warn-log**

Replace lines 76-87 of `src/app_instances/units.rs`:

```rust
        // Determine terrain slope under this entity for tilted VXL rendering.
        // Aircraft fly above terrain and never tilt on slopes.
        let slope_type: u8 = if entity.category == EntityCategory::Aircraft {
            0
        } else {
            state
                .resolved_terrain
                .as_ref()
                .and_then(|t| t.cell(pos.rx, pos.ry))
                .map(|c| if c.slope_type <= 8 { c.slope_type } else { 0 })
                .unwrap_or(0)
        };
```

with:

```rust
        // Determine terrain slope under this entity for tilted VXL rendering.
        // Aircraft fly above terrain and never tilt on slopes. Ground vehicles
        // accept slopes 0-16 (gamemd's full populated range); bytes 17-20
        // collapse to flat at this boundary because gamemd has no matrix
        // populated for them — see VOXEL_SLOPE_TILT_SYSTEM.md addendum.
        let slope_type: u8 = if entity.category == EntityCategory::Aircraft {
            0
        } else {
            state
                .resolved_terrain
                .as_ref()
                .and_then(|t| t.cell(pos.rx, pos.ry))
                .map(|c| {
                    let raw = c.slope_type;
                    if raw <= 16 {
                        raw
                    } else {
                        warn_unexpected_slope_once(raw, pos.rx, pos.ry);
                        0
                    }
                })
                .unwrap_or(0)
        };
```

**Step 3: Verify**

Run:

```
cargo check --lib --package vera20k
```

Expected: clean build.

Run:

```
cargo clippy --lib --package vera20k -- -D warnings
```

Expected: no warnings. `AtomicBool` use is idiomatic; `Ordering::Relaxed`
is correct for a tripwire flag (no synchronization between `WARNED_*`
and any other data).

Run:

```
cargo test --lib --package vera20k app_instances
```

Expected: existing app_instances tests PASS. (No new tests added — the
clamp + warn-log behavior is exercised end-to-end by Task 4.)

**Step 4: Commit**

```
git add src/app_instances/units.rs
git commit -m "app_instances: widen voxel slope clamp to 16, warn on >=17

Bytes 9-16 now reach the atlas key so vehicles render with the
correct gamemd-populated tilt (matching the 9..=16 atlas pre-render
range from the prior commit). Bytes 17-20, which are BSS-zero and
unpopulated in gamemd's slope-matrix table, are clamped to flat at
this boundary.

A one-shot tripwire warns on the first observation of slope_type
>= 17 in the current process so the deferred TMP empirical scan can
be scheduled if it ever fires. The fast path is a single Relaxed
load on AtomicBool, branch-prediction friendly."
```

---

### Task 4: End-to-end verification + visual smoke check

**Why:** Confirm the full pipeline (sim cell → consumer clamp → atlas
key → render) renders slopes 9-16 with the gamemd-correct tilt and
slopes 17-20 as flat (not invisible). This is the integration test that
all three prior tasks build toward.

**Verification: built, tested, log-clean**

Run the full test suite:

```
cargo test --lib --package vera20k
```

Expected: all tests PASS (slope-1-8 tests, the three new slope tests
from Task 1, and unrelated unit_atlas / app_instances tests).

Run a clean build:

```
cargo build --release --package vera20k
```

Expected: clean build, no new warnings.

**Verification: in-game visual smoke check**

The user must run this — it requires the GUI and a sandbox map.

1. Launch the engine:
   ```
   cargo run --release --package vera20k
   ```
2. Load a map with a mix of edge ramps (slopes 1-4), corner ramps (5-8),
   and ideally diagonal-corner ramps (9-16). Most retail YR maps use
   1-8 heavily; for 9-16 coverage a custom test map may be needed if no
   shipping map triggers them. **If the build log shows the line
   `Unit atlas: ... N total needed` with `N` ~89% larger than the
   pre-change baseline, the atlas grew correctly.**
3. Drive a Grizzly or Rhino across each ramp type and observe the tilt
   direction. Compare side-by-side against gamemd.exe on the same map.
   Expected:
   - Slopes 1-8: identical to gamemd (regression test of the prior
     work).
   - Slopes 9-12: tilt visually matches the corresponding 5-8 cell
     (e.g., a slope-9 cell tilts identically to a slope-5 cell).
   - Slopes 13-16: tilt is steeper than 9-12 at the same compass
     direction (EDGE magnitude > CORNER magnitude).
4. Watch the build log for two things:
   - The atlas overflow warning (`unit_atlas.rs:1121-1130`) — if it
     fires, multi-page packing for the unit atlas becomes a follow-up
     task.
   - The `slope_type N encountered at cell (rx, ry)` warn-log — if it
     fires, schedule the deferred TMP empirical scan.

**Optional: confirm the atlas grew**

Before the change, log the line `"Unit atlas: X cached, Y new to render,
Z total needed"` from `unit_atlas.rs:468-473` for a typical map. After
the change, the same map should show `Z` roughly 1.89× larger. Capture
both numbers in the commit message or in `ra2-rust-game-docs/` if useful
for future scale planning.

**No commit in this task** — Tasks 1, 2, and 3 each ended with their own
atomic commit. This task is verification only.

---

## Sources & References

- **Design doc:** [docs/plans/2026-05-10-voxel-slope-tilt-9-20-design.md](docs/plans/2026-05-10-voxel-slope-tilt-9-20-design.md)
- **Primary Ghidra report:** `ra2-rust-game-docs/VOXEL_SLOPE_TILT_SYSTEM.md`,
  specifically the "Slope Matrix Table — Full Entry List" addendum
  (verified 2026-05-10).
- **Older parent doc (lower detail, superseded for 9+):**
  `ra2-rust-game-docs/VXL_DRAW_MATRIX_GHIDRA_REPORT.md`.
- **Tangentially related (TMP byte → cell propagation, not vehicle
  tilt):** `ra2-rust-game-docs/LAT_GROUPS_AND_SLOPE_FIXUP_GHIDRA_REPORT.md`.
- **gamemd.exe addresses (kept here, not in Rust comments):**
  - `VXL_MasterLighting_Init` @ `0x00754CB0` — runtime populator of the slope-matrix table.
  - `VXL_GetFacingMatrix` @ `0x007559B0` — table lookup; no bounds clamp.
  - `Matrix3x4_BuildFromRotateXAndFacing` @ `0x005AE6F0` — Rz·Rx·Rz⁻¹ builder.
  - `DAT_00b45188` — table base; `DAT_00b454B8` — slot for slope_type=17 (BSS-zero).
  - Tilt constants: `_DAT_00b44310` (EDGE = 0.5214767), `_DAT_00b43f08` (CORNER = 0.3858827).
  - 8 IEEE-754 compass literals at `0x00000000`, `0x3F490E56`, `0x3FC90E56`,
    `0x4016CAC1`, `0x40490E56`, `0x407B51EC`, `0x4096CAC1`, `0x40AFEC8B`.
- **INI keys:** None. Slope tilt is TMP-driven (file format), not INI-driven.
- **Related code (existing patterns mirrored):**
  - `src/render/vxl_raster.rs:49,57` — `EDGE_TILT_RAD`/`CORNER_TILT_RAD` constants
    (already verified by formula tripwire tests at `:799-826`).
  - `src/render/vxl_raster.rs:255-273` — slopes-1-8 match arm (extended).
  - `src/render/vxl_raster.rs:828-860` — slope-4 geometry lockdown test (mirrored style).
  - `src/render/unit_atlas.rs:210-211, 333-334, 432-433` — three pre-render
    call sites (widened).
  - `src/app_instances/units.rs:81-87` — consumer clamp (widened).
  - `src/app_instances/units.rs:281-298` — atlas-miss → slope_type=0 fallback
    (already in place; safety net retained).
- **Prior commits this work builds on:**
  - `169d42a` — render/vxl: correct slope tilt constants to gamemd-verified values.
  - `0f2fa4a` — render/vxl: correct numeric drift in tilt constants + tripwire tests.
  - `af411d5` — render/vxl: geometry test locks current tilt direction for slope_type=4.
