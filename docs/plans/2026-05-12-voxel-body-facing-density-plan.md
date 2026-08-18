# Voxel Body Facing Density — Phase 1 of Approach E

> **For Claude:** Execute task-by-task. Each task is self-contained.

**Goal:** Halve the body/composite facing bucket size from step=4 (64 buckets, 5.6°) to step=2 (128 buckets, 2.8°) to reduce visible rotation-snap on voxel vehicles, matching the existing turret/barrel facing density.

**Architecture:** Two `const u8` values in `src/render/unit_atlas.rs` drive the entire bake enumeration. All consumers route through `canonical_unit_facing()` and `facing_config_for_layer()`, so the change is data-driven — no algorithmic rewrites. The eager preload at level start (`collect_needed_unit_keys`) already enumerates via these helpers, so no preload-path changes are needed. The risk surface is (a) memory budget and (b) tests/tools that hardcode the old values in parallel.

**Design Source:** In-conversation brainstorm — Approach E from the voxel orientation-coverage discussion. Phase 2 (runtime voxel render for rocking, plus damage/death tilt) is deferred pending `/re-investigate` on rocking constants and damage state mechanism.

---

## Grounding Summary

- **ra2-rust-game-docs/**: no facing-density research exists; this is an internal engine policy decision, not gamemd parity. gamemd renders voxels at the exact `DirStruct` angle every frame (effectively continuous to 256 levels). Our sprite atlas is an optimization, and step=2 is a quality improvement on top of that optimization, not a parity-driven change.
- **Ghidra MCP**: not needed. The change does not implement or mimic any gamemd code path.
- **Repo pattern**: `TURRET_FACING_STEP = 2` / `TURRET_FACING_BUCKETS = 128` ([src/render/unit_atlas.rs:42-44](src/render/unit_atlas.rs#L42-L44)) is the exact pattern we are extending to body/composite layers. No new pattern introduced.
- **INI keys**: none. Facing quantization is not parameterized by INI.
- **Still unknown**: actual atlas memory footprint at 30-player saturation after the change — must be measured via `measure-atlas` binary before the change is committed.

## Key Technical Decisions

- **Step = 2, not step = 1.** Step=2 matches the existing turret resolution and doubles body fineness; step=1 (256 buckets, 1.4°) would quadruple it but is near the limit of human-perceptible iso rotation smoothness at typical RA2 zoom levels. **Confidence:** medium — perceptual call, not measured. **Source:** brainstorm decision; can revisit with a side-by-side playtest if step=2 still feels snappy.
- **Update measure-atlas in lockstep with the constants**, not via a `pub const` re-export from unit_atlas, because the existing measurement tool already chose a hardcoded approach. Changing that convention is out of scope. **Confidence:** high. **Source:** existing pattern in [src/bin/measure-atlas.rs:25-28](src/bin/measure-atlas.rs#L25-L28).
- **Don't change `TURRET_FACING_STEP`** in this plan. Turrets are already at step=2; pushing them to step=1 was offered as an option in the brainstorm but not picked. **Confidence:** high. **Source:** brainstorm scope.

## Open Questions

### Resolved During Planning

- **Do consumers hardcode step=4 or bucket counts?** Audited via grep — `canonical_unit_facing` and `facing_config_for_layer` are the only entry points, and both auto-adapt to the constants. Two outliers: `unit_atlas_tests.rs` (hardcoded assertions) and `bin/measure-atlas.rs` (hardcoded `BODY_FACING_BUCKETS=64`). Both addressed in tasks below.
- **Stale doc comments on canonical_*_facing?** Confirmed stale (comments say "32 buckets (step=8)" — wrong even today). Will refresh.

### Deferred to Implementation

- **Final atlas size at 30-player saturation.** Measured in Task 4; the plan adapts if it exceeds the max texture dimension or pushes the per-build memory budget noticeably past 200 MB.
- **Visible smoothness improvement at step=2.** Can only be evaluated in-game (Task 6). If still snappy, follow-up is step=1 for body, or jump to Phase 2 (runtime voxel rendering for actively rotating units).

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | [src/render/unit_atlas.rs:32-38](src/render/unit_atlas.rs#L32-L38) | Flip `UNIT_FACING_STEP` and `UNIT_FACING_BUCKETS`; refresh doc comment block |
| Modify | [src/render/unit_atlas.rs:1006-1017](src/render/unit_atlas.rs#L1006-L1017) | Refresh stale doc comments on `canonical_unit_facing` and `canonical_turret_facing` |
| Modify | [src/render/unit_atlas_tests.rs:164](src/render/unit_atlas_tests.rs#L164) | Update `canonical_unit_facing(3) == 0` assertion |
| Modify | [src/render/unit_atlas_tests.rs:169-176](src/render/unit_atlas_tests.rs#L169-L176) | Update `test_facing_config_for_layer` Body/Composite expectations |
| Modify | [src/bin/measure-atlas.rs:25](src/bin/measure-atlas.rs#L25) | Bump `BODY_FACING_BUCKETS` from 64 to 128 |

## Interface Changes

None. The public functions `canonical_unit_facing` and `facing_config_for_layer` keep the same signature; their return values change behaviorally but their contract ("quantize to the configured bucket") is unchanged.

## Sim Checklist

N/A — no `sim/` changes. Render-layer constants only.

## Risk Areas

- **Atlas memory growth.** Body sprite count doubles. Worst case at 30-player saturation grows from ~102 MB body-sprite portion to ~204 MB. Total atlas may exceed measure-atlas's "stays under 200 MB" claim. Mitigation: measure first (Task 4); if over budget, document the new ceiling and accept it (we have multi-page support per [feedback_multi_atlas memory](file:///<local>/.claude/projects/<claude-project>/memory/feedback_multi_atlas.md), though the current unit atlas is single-page).
- **Max texture dimension.** The shelf-packer at [unit_atlas.rs:1052-1090](src/render/unit_atlas.rs#L1052-L1090) widens the atlas if height exceeds GPU limit, but if width also hits the limit it logs a warning and truncates. If we hit that path, sprites will silently drop. Verified via Task 4 measurement.
- **Existing pre-baked atlases on disk (if any).** Not applicable — atlas is built at level load each session, no on-disk cache.

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|--------|------|----------------|--------------|
| Task 6 | Rotation smoothness vs gamemd | gamemd renders voxels at continuous angle. Step=2 is closer to indistinguishable than step=4 was, but neither is *truly* continuous. Player-visible during every rotation, every match. | Side-by-side playtest: rotate a Grizzly through a full circle in both gamemd and our engine; the staircase should be perceptibly smaller after the change. Not pixel-exact — gamemd is continuous, we are quantized — but visibly closer. |

Phase 2 (runtime rendering of actively rotating vehicles) is the gamemd-parity end state. Phase 1 is an interim closeness improvement, not a parity claim.

---

## Tasks

### Task 1: Update facing constants and refresh inline comments

**Why:** The two constants drive the entire bake enumeration. Update them first; everything downstream auto-adapts via `canonical_unit_facing` and `facing_config_for_layer`.

**Files:**
- Modify: [src/render/unit_atlas.rs:32-44](src/render/unit_atlas.rs#L32-L44)

**Pattern:** Mirrors the existing TURRET_FACING_STEP / TURRET_FACING_BUCKETS block at lines 39-44.

**Step 1: Edit constants and comments**

Replace lines 32-44 with:

```rust
/// Body/composite facing quantization step: 2 = 128 buckets (2.8° per bucket).
/// Doubled from 64→128 to bring body smoothness in line with the existing
/// turret/barrel resolution. Eliminates the visible staircase that was
/// noticeable during rotation at step=4 (5.6° per bucket).
const UNIT_FACING_STEP: u8 = 2;
/// Number of pre-rendered facing directions for body/composite sprites.
const UNIT_FACING_BUCKETS: u8 = 128;
/// Turret/barrel facing quantization step: 2 = 128 buckets (2.8° per bucket).
/// Turrets rotate frequently during combat, so finer resolution prevents
/// visible stepping.
const TURRET_FACING_STEP: u8 = 2;
/// Number of pre-rendered facing directions for turret/barrel sprites.
const TURRET_FACING_BUCKETS: u8 = 128;
```

**Step 2: Refresh stale doc comments on the canonical_* functions**

At [src/render/unit_atlas.rs:1006-1017](src/render/unit_atlas.rs#L1006-L1017), replace:

```rust
/// Canonicalize body/composite facing to one of 32 rendered facing buckets (step=8).
pub fn canonical_unit_facing(facing: u8) -> u8 {
    (facing / UNIT_FACING_STEP) * UNIT_FACING_STEP
}

/// Canonicalize turret/barrel facing to one of 64 rendered facing buckets (step=4).
/// Accepts 16-bit DirStruct, converts to 8-bit for sprite frame selection.
/// This is the single u16→u8 conversion point for turret rendering.
pub fn canonical_turret_facing(facing_u16: u16) -> u8 {
```

With:

```rust
/// Canonicalize body/composite facing to one of `UNIT_FACING_BUCKETS` buckets.
pub fn canonical_unit_facing(facing: u8) -> u8 {
    (facing / UNIT_FACING_STEP) * UNIT_FACING_STEP
}

/// Canonicalize turret/barrel facing to one of `TURRET_FACING_BUCKETS` buckets.
/// Accepts 16-bit DirStruct, converts to 8-bit for sprite frame selection.
/// This is the single u16→u8 conversion point for turret rendering.
pub fn canonical_turret_facing(facing_u16: u16) -> u8 {
```

(Drop the hardcoded "32 buckets / 64 buckets" numbers from the doc comments — reference the constants instead so they can't go stale again.)

**Step 3: Verify**

Run: `cargo check`

Expected: Compiles. Pre-existing warnings only.

### Task 2: Update unit atlas tests for new bucket size

**Why:** Tests hardcode step=4 / buckets=64. They will fail after Task 1.

**Files:**
- Modify: [src/render/unit_atlas_tests.rs:164](src/render/unit_atlas_tests.rs#L164)
- Modify: [src/render/unit_atlas_tests.rs:169-180](src/render/unit_atlas_tests.rs#L169-L180)

**Pattern:** Updating tests to reflect new constants. Same structure as existing turret tests.

**Step 1: Read the current test bodies**

Read [src/render/unit_atlas_tests.rs:145-185](src/render/unit_atlas_tests.rs#L145-L185) to see the full test functions and any other hardcoded values nearby.

**Step 2: Update the comment-anchor on line 163 and the body assertion on line 164**

Replace:

```rust
    // Verify finer than body facing (step=4).
    assert_eq!(canonical_unit_facing(3), 0); // snaps to 0
```

With:

```rust
    // Verify body and turret facing share the same step granularity.
    assert_eq!(canonical_unit_facing(3), 2); // step=2, snaps 3 to 2
```

**Step 3: Update `test_facing_config_for_layer` Body and Composite branches**

Replace lines 169-176 (the Body and Composite assertions only — leave Turret/Barrel assertions intact):

```rust
fn test_facing_config_for_layer() {
    let (step, buckets) = super::facing_config_for_layer(VxlLayer::Body);
    assert_eq!(step, 4);
    assert_eq!(buckets, 64);

    let (step, buckets) = super::facing_config_for_layer(VxlLayer::Composite);
    assert_eq!(step, 4);
    assert_eq!(buckets, 64);
```

With:

```rust
fn test_facing_config_for_layer() {
    let (step, buckets) = super::facing_config_for_layer(VxlLayer::Body);
    assert_eq!(step, 2);
    assert_eq!(buckets, 128);

    let (step, buckets) = super::facing_config_for_layer(VxlLayer::Composite);
    assert_eq!(step, 2);
    assert_eq!(buckets, 128);
```

**Step 4: Verify**

Run: `cargo test --lib unit_atlas_tests`

Expected: All tests pass.

### Task 3: Update measure-atlas hardcoded bucket count

**Why:** The measurement tool's `BODY_FACING_BUCKETS` constant is parallel-tracked with the unit_atlas constant. If we don't update it, Task 4's measurement will be wrong (it will report old-baseline numbers, not the new ones).

**Files:**
- Modify: [src/bin/measure-atlas.rs:24-25](src/bin/measure-atlas.rs#L24-L25)

**Pattern:** Mirrors the existing TURRET_FACING_BUCKETS constant just below.

**Step 1: Edit the constant**

Replace lines 24-25:

```rust
/// Body/composite facing buckets (matches `unit_atlas::UNIT_FACING_BUCKETS`).
const BODY_FACING_BUCKETS: usize = 64;
```

With:

```rust
/// Body/composite facing buckets (matches `unit_atlas::UNIT_FACING_BUCKETS`).
const BODY_FACING_BUCKETS: usize = 128;
```

**Step 2: Verify**

Run: `cargo check --bin measure-atlas`

Expected: Compiles.

### Task 4: Measure atlas memory at saturation

**Why:** Body sprite count doubles. We need to know the new worst-case footprint before claiming the change is safe to ship.

**Files:** none modified

**Step 1: Run the measurement**

Run: `cargo run --release --bin measure-atlas`

**Step 2: Record the output**

Read the printed totals. Compare to the previous baseline (the binary's comment claims < 200 MB).

**Step 3: Decision gate**

- If new total ≤ ~250 MB: acceptable; proceed to Task 5.
- If new total > 250 MB but under a single-atlas-page cap (max_texture_dimension² × 1 byte): acceptable but worth noting; proceed to Task 5 and add a one-line note to the next commit body about the new ceiling.
- If new total exceeds what a single atlas page can hold (atlas would either truncate or fail to pack): **STOP**. Hand back to user — the change cannot ship until either (a) we add multi-page support to UnitAtlas or (b) we walk back the step=2 decision to step=3 or step=4 with a different mitigation.

### Task 5: Full build verification

**Why:** Ensure nothing else broke.

**Files:** none modified.

**Step 1: Run check**

Run: `cargo check --message-format=short 2>&1 | tail -20`

Expected: Compiles. Pre-existing warnings only — no new warnings from the change.

**Step 2: Run library tests**

Run: `cargo test --lib`

Expected: All tests pass.

### Task 6: Playtest verification

**Why:** The whole point of the change is observable smoothness. Code passing tests is not the same as the player seeing the improvement.

**Files:** none modified

**Step 1: Spawn a turreted vehicle in the engine**

Launch the game. Get a turreted vehicle (Grizzly, Rhino, Flak Track — any) into play.

**Step 2: Slowly rotate the unit through a full 360°**

Best test: order it to follow a circular waypoint, or rotate it in place by ordering it to face different targets.

**Step 3: Observe**

- The visible staircase from rotation should be noticeably smaller than before.
- Body and turret should appear to rotate at similar smoothness (both at step=2 now).
- Confirm no white patches return (depth fix should be unaffected — same atlas keys, same depth path).

**Step 4: Decision gate**

- If smoothness is meaningfully improved: success. Proceed to Task 7.
- If still visibly snappy: not necessarily a fix failure — may indicate we want step=1 or Phase 2 acceleration. Discuss with user before continuing.

### Task 7: Commit

**Why:** Ship the change atomically with its test updates.

**Step 1: Stage and commit**

```
git add src/render/unit_atlas.rs src/render/unit_atlas_tests.rs src/bin/measure-atlas.rs
git commit -m "render/unit_atlas: double body facing density to step=2 (128 buckets)

Reduces visible rotation-snap on voxel vehicles by halving the body/
composite facing bucket size from 5.6° to 2.8°, matching the existing
turret/barrel resolution. Phase 1 of Approach E.

Atlas memory grows by [N] MB at 30-player saturation per measure-atlas;
remains within budget."
```

(Substitute the actual measured value from Task 4 into the commit body.)

**Step 2: Verify**

Run: `git status` — clean working tree.

## Sources & References

- **Brainstorm context (in-conversation):** Approach E from the voxel orientation-coverage discussion; Phase 1 = denser body facings.
- **Repo patterns:**
  - [src/render/unit_atlas.rs:42-44](src/render/unit_atlas.rs#L42-L44) — TURRET_FACING_STEP=2 / BUCKETS=128, the exact pattern being mirrored
  - [src/render/unit_atlas.rs:173-228](src/render/unit_atlas.rs#L173-L228) — `collect_needed_unit_keys`, auto-adapts to the new bucket count via `facing_config_for_layer`
  - [src/render/unit_atlas.rs:1052-1090](src/render/unit_atlas.rs#L1052-L1090) — atlas packer's widen-on-overflow retry loop
- **Related memories:**
  - `feedback_multi_atlas.md` — multi-page atlas support exists (currently used for SHP atlas; not yet for unit atlas)
  - `project_scale_target.md` — 20k units, 30 players target; this change scales body-sprite cost linearly with that ceiling
- **gamemd.exe addresses:** none. Phase 1 is not a gamemd-parity change at the function-by-function level — gamemd renders continuous, we are just bringing our quantization closer.
- **INI keys:** none.
