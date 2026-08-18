# Garrison Frame Swap Implementation Plan

> **For Claude:** Execute this plan task-by-task. Each task is self-contained.

**Goal:** Make `CanBeOccupied=yes` building bodies render the correct SHP frame based on occupant count and damage tier, matching gamemd.exe's `BuildingClass::GetCurrentFrame` branch 4 formula.

**Architecture:** Render-side change. A pure helper function `building_frame_index` lives in [src/app_instances/shp.rs](../../src/app_instances/shp.rs), called from the existing frame-selection match for `EntityCategory::Structure`. The atlas builder ([src/render/sprite_atlas.rs](../../src/render/sprite_atlas.rs)) is extended to register frames 0..3 for `can_be_occupied=true` buildings instead of frame 0 only. No sim changes; determinism contract preserved.

**Design Doc:** [docs/plans/2026-05-05-garrison-frame-swap-design.md](2026-05-05-garrison-frame-swap-design.md)

---

## Grounding Summary

**Docs (R1):** [GARRISON_FRAME_SWAP_GHIDRA_REPORT.md](../../../ra2-rust-game-docs/GARRISON_FRAME_SWAP_GHIDRA_REPORT.md) — high-confidence formula and field map. Predecessor: [BUILDINGCLASS_DRAWBODY_GHIDRA_REPORT.md](../../../ra2-rust-game-docs/BUILDINGCLASS_DRAWBODY_GHIDRA_REPORT.md) §3.2 (parent doc, contains a mislabeled "OccupantCount > 0" yellow gate that the new report corrects). Supporting: [TECHNOTYPECLASS_BASE_GHIDRA_REPORT.md](../../../ra2-rust-game-docs/TECHNOTYPECLASS_BASE_GHIDRA_REPORT.md) (TechLevel @ +0x634), [GARRISON_OCCUPANT_SYSTEM_GHIDRA_REPORT.md](../../../ra2-rust-game-docs/GARRISON_OCCUPANT_SYSTEM_GHIDRA_REPORT.md) (occupant count @ +0x694).

**Ghidra verification (R2):** Decompiled `BuildingClass::GetCurrentFrame` at `0x0043EF90` directly. Verified `vtable+0x84 = GetTechnoType`, `vtable+0x408 = GetOccupantCount`, `vtable+0x184 = GetCurrentMission`. Confirmed `Rules+0x1700 = ConditionYellow` and `Rules+0x1708 = ConditionRed` via `RulesClass::ReadAudioVisual` string xrefs. Confirmed `BuildingClass+0x534` initializes to -1 in the constructor (`OR EAX, -1` at `0x0043B770`), meaning civilian map-placed buildings stay at -1 and branch 4 fires unconditionally for them — this is what makes the formula visible at full HP for civilians.

**Repo pattern (R3):** Mirrors [src/app_ui_overlays.rs:766-769](../../src/app_ui_overlays.rs#L766) `health_pip_variant` — a render-side pure helper taking `(ratio, condition_yellow, condition_red)` and returning a u32 frame index. Same shape, same module style. Sister stub already in shp.rs at line 138 — drop the new helper next to `resolve_infantry_shp_frame` at line 572.

**INI keys (R4):** No new parsing required. `CanBeOccupied`, `TechLevel`, `[AudioVisual] ConditionYellow`, `[AudioVisual] ConditionRed` all already parsed. 95 garrisonable types in `rulesmd.ini` use TechLevel=-1 (verified spot-checks: CABHUT, CALA01, CAGAS01, CABUNK01-02). All have ≥4-frame SHPs (assumption validated by Approach A in the design doc).

**Still unknown:** Whether a healthy garrisoned civilian building visibly swaps to frame 2 in the original gamemd.exe (~85% confidence yes). The fallback to frame 0 in the renderer means we degrade gracefully if wrong; a one-line gate could be added later if observation contradicts the assumption.

## Key Technical Decisions

- **Apply formula unconditionally for `can_be_occupied=true` buildings** (no BState/damage gate): **Confidence:** medium-high
  - **Source:** GARRISON_FRAME_SWAP_GHIDRA_REPORT.md §3.3 + binary `0x0043B770` (constructor sets +0x534=-1, branch 4 fires by default for civilians). Open Question #1 in the report.

- **Use floats for the health ratio in the helper:** **Confidence:** high
  - **Source:** CLAUDE.md "float is fine for rendering math in glam"; precedent at [src/app_ui_overlays.rs:766](../../src/app_ui_overlays.rs#L766) already uses `f32` ratio in render-side health pip selection.

- **Atlas registration limited to `can_be_occupied=true`:** **Confidence:** high
  - **Source:** Brainstorm Q1 = b. Avoids registering frames 1-3 for ~hundreds of non-garrisonable buildings.

- **Fallback to frame 0 in renderer when atlas lookup misses (Approach A):** **Confidence:** high
  - **Source:** Brainstorm Step 5 = A. All standard YR civilian garrisonable SHPs verified to have ≥4 frames.

## Open Questions

### Resolved During Planning

- "Where does the helper live?" → Inline private fn in `src/app_instances/shp.rs`, sibling of `resolve_infantry_shp_frame`. Confirmed by brainstorm Q3 = b.
- "Should we add `condition_yellow_x1000`?" → No. Render layer can use floats directly; the integer-scaled threshold is only needed for sim determinism (per `passenger.rs:213`).
- "Does `tech_level` need parsing?" → Already parsed at [object_type.rs:693](../../src/rules/object_type.rs#L693) with default -1.

### Deferred to Implementation

- **Visual confirmation in-game** that healthy garrisoned civilians show frame 2. To answer: load a skirmish, garrison a CABHUT/CABNK01 with infantry, observe. If frame 0 stays, wrap the helper call in a `health_ratio <= condition_yellow` gate.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | `src/app_instances/shp.rs` | Add `building_frame_index` helper; replace `Structure → (0, None)` stub; add atlas-lookup fallback for garrisonables |
| Modify | `src/render/sprite_atlas.rs` | Extend `EntityCategory::Structure` arm at line 285 to register frames 1-3 when `can_be_occupied=true` |

No new files. Helper is module-private to `app_instances/shp.rs`.

## Interface Changes

None. The helper is private; the public `build_shp_instances` and `build_sprite_atlas` signatures are unchanged.

## Sim Checklist

Not applicable — no sim/ changes. Frame index is render-side. Sim state hash unaffected.

## Risk Areas

| Risk | Mitigation |
|---|---|
| Atlas page count grows past current capacity | Memory `feedback_multi_atlas` already documents multi-page atlas support. Spot-check page count after change. |
| SHP with fewer than 4 frames for a garrisonable building | Approach A: renderer falls back to frame 0. Verified all standard YR civilian garrisonable SHPs have ≥4 frames. |
| Healthy garrisoned civilian doesn't actually swap to frame 2 in gamemd | One-line fix to wrap helper call in damage gate. Documented in Open Questions. |
| Atlas-lookup-fallback path masks SHP defects | Per memory `feedback_silent_render_failures`: fallback applies ONLY when `can_be_occupied=true` AND `frame != 0`. Non-garrisonable buildings still skip + warn via existing path. |

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|--------|------|----------------|--------------|
| Task 4 | Frame index for occupied civilian at full HP | Player garrisons CABHUT → building should show "lit windows" frame 2, not stay at frame 0. Visible every match where civilians are garrisoned. | Garrison a CABHUT in skirmish; compare to gamemd.exe. Per GARRISON_FRAME_SWAP_GHIDRA_REPORT.md §3.4 Open Question #1. |
| Task 4 | Frame collapse: occupied + red HP → frame 1 | Civilian SHPs only have 3 effective frames; frame 3 doesn't exist. Without collapse, atlas miss → fallback to frame 0 (wrong-looking empty). With collapse, draws frame 1 (red-damaged empty look). | Damage a garrisoned CABHUT to <25% HP; compare to gamemd.exe. |
| Task 4 | Frame index for empty civilian at red HP | Civilian SHPs swap to "destroyed" look (frame 1) at red. Visible whenever a civilian building takes near-fatal damage. | Damage an empty CABHUT to <25% HP; compare to gamemd.exe. |

---

## Tasks

### Task 1: Add the `building_frame_index` helper

**Why:** The pure formula needs to exist before any caller invokes it. Defining and implementing the helper in one task because the body is ~15 lines — splitting into two would be overkill.

**Files:**
- Modify: [src/app_instances/shp.rs](../../src/app_instances/shp.rs) — add new fn near `resolve_infantry_shp_frame` (around line 570)

**Pattern:** Mirrors [src/app_ui_overlays.rs:766-769](../../src/app_ui_overlays.rs#L766) `health_pip_variant` — render-side pure helper, takes ratio + thresholds, returns frame index.

**Step 1: Append the helper at the end of `src/app_instances/shp.rs` (after `resolve_infantry_shp_frame`)**

```rust
/// Compute the body SHP frame index for a `CanBeOccupied=yes` building.
///
/// Implements the formula from `BuildingClass::GetCurrentFrame` branch 4 (gamemd.exe
/// 0x0043EF90). For civilian buildings (`tech_level == -1`) the yellow-tier damage
/// step is skipped, and the (occupied, red-HP) collapse rule maps frame 3 → frame 1
/// so 3-frame civilian SHPs render correctly.
///
/// Inputs are owned primitives so this is trivially unit-testable.
fn building_frame_index(
    occupant_count: u32,
    health_current: u16,
    health_max: u16,
    tech_level: i32,
    condition_yellow: f32,
    condition_red: f32,
) -> u16 {
    let mut base: u16 = 0;
    if occupant_count > 0 {
        base = 2;
    }
    let ratio = if health_max == 0 {
        1.0
    } else {
        health_current as f32 / health_max as f32
    };
    let red_tier = ratio <= condition_red;
    let yellow_tier = tech_level > 0 && ratio <= condition_yellow;
    if red_tier || yellow_tier {
        base += 1;
    }
    if tech_level == -1 && base == 3 {
        return 1;
    }
    base
}
```

**Step 2: Verify build**

Run: `cargo check --lib`
Expected: PASS (no warnings related to the new fn since it's unused at this point — that's fine, the next task adds the call site).

**Step 3: Commit**

```
render: add building_frame_index helper for CanBeOccupied frame swap
```

---

### Task 2: Add the unit tests

**Why:** Lock the formula's truth table before any caller depends on it. The tests are pure-logic and self-contained.

**Files:**
- Modify: [src/app_instances/shp.rs](../../src/app_instances/shp.rs) — extend the existing `#[cfg(test)] mod tests` block at line 592 (which already contains the `resting_building_anim_frame` tests). Do NOT create a new `mod tests` — that produces a duplicate-definition compile error.

**Pattern:** Standard Rust unit-test module convention; same shape as test blocks elsewhere in the repo.

**Step 1: Extend the existing `mod tests` block in `src/app_instances/shp.rs`**

(a) At line 594, alongside the existing `use super::resting_building_anim_frame;`, add:

```rust
use super::building_frame_index;
```

(b) Append the following test fns inside the existing `mod tests { ... }` block (after the last existing test, before the closing `}`):

```rust
    // Civilian (TechLevel == -1) — matches CABHUT, CALA01, CAGAS01, CABUNK01, etc.
    // Yellow-tier damage step is gated on TechLevel > 0, so it never fires here.
    // Frame 3 collapses to 1 (occupied + red).

    #[test]
    fn civilian_empty_healthy_returns_0() {
        assert_eq!(building_frame_index(0, 100, 100, -1, 0.5, 0.25), 0);
    }

    #[test]
    fn civilian_empty_yellow_tier_returns_0() {
        // ratio = 0.4: below ConditionYellow but above ConditionRed.
        // Yellow gate is `tech_level > 0` — fails for civilian, so no +1.
        assert_eq!(building_frame_index(0, 40, 100, -1, 0.5, 0.25), 0);
    }

    #[test]
    fn civilian_empty_red_tier_returns_1() {
        assert_eq!(building_frame_index(0, 20, 100, -1, 0.5, 0.25), 1);
    }

    #[test]
    fn civilian_occupied_healthy_returns_2() {
        assert_eq!(building_frame_index(1, 100, 100, -1, 0.5, 0.25), 2);
    }

    #[test]
    fn civilian_occupied_yellow_tier_returns_2() {
        // Same yellow-gate behavior as empty case.
        assert_eq!(building_frame_index(1, 40, 100, -1, 0.5, 0.25), 2);
    }

    #[test]
    fn civilian_occupied_red_tier_collapses_to_1() {
        // base=2 (occupied) + 1 (red) = 3 → collapse rule → 1.
        assert_eq!(building_frame_index(1, 20, 100, -1, 0.5, 0.25), 1);
    }

    // Buildable (TechLevel >= 1) — TS-era "buildable garrisonable" structures
    // (none in standard YR but the formula path is real). Yellow tier fires.

    #[test]
    fn buildable_empty_healthy_returns_0() {
        assert_eq!(building_frame_index(0, 100, 100, 5, 0.5, 0.25), 0);
    }

    #[test]
    fn buildable_empty_yellow_tier_returns_1() {
        assert_eq!(building_frame_index(0, 40, 100, 5, 0.5, 0.25), 1);
    }

    #[test]
    fn buildable_occupied_healthy_returns_2() {
        assert_eq!(building_frame_index(1, 100, 100, 5, 0.5, 0.25), 2);
    }

    #[test]
    fn buildable_occupied_red_tier_returns_3() {
        // No civilian collapse (tech_level != -1).
        assert_eq!(building_frame_index(1, 20, 100, 5, 0.5, 0.25), 3);
    }

    // Edge cases.

    #[test]
    fn zero_max_hp_treats_as_healthy() {
        // Avoids division-by-zero; entity not yet fully initialized.
        assert_eq!(building_frame_index(0, 0, 0, -1, 0.5, 0.25), 0);
    }

    #[test]
    fn boundary_at_condition_red_inclusive() {
        // ratio == ConditionRed exactly → red_tier fires (<=).
        assert_eq!(building_frame_index(0, 25, 100, -1, 0.5, 0.25), 1);
    }
```

(Note: do NOT include a closing `}` after the last `#[test]` — the existing
`mod tests` block at line 592 already has its own closing `}` at the end of
the file; you're inserting these `#[test]` items inside that block.)

**Step 2: Run the tests**

Run: `cargo test -p ra2-rust-game --lib building_frame_index`

Expected: 12 tests pass. (The exact crate name is whatever `Cargo.toml` declares — adjust if needed; if the crate has only one binary, plain `cargo test --lib building_frame_index` works.)

**Step 3: Commit**

```
render: unit tests for building_frame_index (12 cases, civilian + buildable + edges)
```

---

### Task 3: Extend atlas registration for `can_be_occupied` buildings

**Why:** Frame indices 1, 2, 3 must exist in the atlas before the renderer can request them. Without this, every garrisonable building lookup for a non-zero frame returns None and falls back via Task 5.

**Files:**
- Modify: [src/render/sprite_atlas.rs:283-291](../../src/render/sprite_atlas.rs#L283) — extend the `EntityCategory::Structure` match arm in `build_sprite_atlas`'s `needed`-collection loop.

**Pattern:** Same `needed.insert(ShpSpriteKey { ... })` calls as the existing line 285. We just emit four keys instead of one for `can_be_occupied=true` types.

**Step 1: Replace lines 283-291**

Locate this block:

```rust
        match entity.category {
            EntityCategory::Structure => {
                needed.insert(ShpSpriteKey {
                    type_id: type_str.to_string(),
                    facing: 0,
                    frame: 0,
                    house_color: color_idx,
                });
            }
```

Replace with:

```rust
        match entity.category {
            EntityCategory::Structure => {
                needed.insert(ShpSpriteKey {
                    type_id: type_str.to_string(),
                    facing: 0,
                    frame: 0,
                    house_color: color_idx,
                });
                // CanBeOccupied buildings need frames 0..3 for the occupancy +
                // damage-tier frame swap (see building_frame_index in
                // app_instances/shp.rs). SHPs with fewer frames silently skip
                // missing entries; the renderer falls back to frame 0.
                let can_be_occupied = rules
                    .and_then(|r| r.object(type_str))
                    .map(|obj| obj.can_be_occupied)
                    .unwrap_or(false);
                if can_be_occupied {
                    for frame in 1u16..=3 {
                        needed.insert(ShpSpriteKey {
                            type_id: type_str.to_string(),
                            facing: 0,
                            frame,
                            house_color: color_idx,
                        });
                    }
                }
            }
```

**Step 2: Verify build**

Run: `cargo check --lib`
Expected: PASS.

**Step 3: Commit**

```
atlas: register frames 0..3 for CanBeOccupied buildings
```

---

### Task 4: Replace the Structure frame stub with the helper call

**Why:** This is the single change that makes the formula take effect at draw time.

**Files:**
- Modify: [src/app_instances/shp.rs:138-149](../../src/app_instances/shp.rs#L138) — replace the `Structure => (0, None)` arm.

**Pattern:** Inline match arm, mirrors the surrounding `is_building_up`/`is_building_down` branches that compute frames from sim state.

**Step 1: Replace lines 138-149**

Locate this block:

```rust
        } else {
            match entity.category {
                EntityCategory::Structure => (0, None),
                _ => (
                    resolve_infantry_shp_frame(
                        state,
                        type_str,
                        entity.facing,
                        entity.animation.as_ref(),
                    ),
                    None,
                ),
            }
        };
```

Replace with:

```rust
        } else {
            match entity.category {
                EntityCategory::Structure => {
                    let obj = state.rules.as_ref().and_then(|r| r.object(type_str));
                    let frame = if obj.map(|o| o.can_be_occupied).unwrap_or(false) {
                        let occupant_count = entity
                            .passenger_role
                            .cargo()
                            .map(|c| c.count())
                            .unwrap_or(0);
                        let tech_level = obj.map(|o| o.tech_level).unwrap_or(-1);
                        let (cy, cr) = state
                            .rules
                            .as_ref()
                            .map(|r| (r.general.condition_yellow, r.general.condition_red))
                            .unwrap_or((0.5, 0.25));
                        building_frame_index(
                            occupant_count,
                            entity.health.current,
                            entity.health.max,
                            tech_level,
                            cy,
                            cr,
                        )
                    } else {
                        0
                    };
                    (frame, None)
                }
                _ => (
                    resolve_infantry_shp_frame(
                        state,
                        type_str,
                        entity.facing,
                        entity.animation.as_ref(),
                    ),
                    None,
                ),
            }
        };
```

**Step 2: Verify build**

Run: `cargo check --lib`
Expected: PASS. The helper signature accepts `u32` for `occupant_count`, matching `PassengerCargo::count()` at [src/sim/passenger.rs:58](../../src/sim/passenger.rs#L58); no cast required.

**Step 3: Commit**

```
render: wire building_frame_index into Structure frame selection
```

---

### Task 5: Add atlas-lookup fallback for garrisonable buildings

**Why:** Approach A: if a civilian SHP has fewer than 4 frames, the atlas registration in Task 3 will silently skip the missing ones. Without this fallback, the renderer's existing `let Some(entry) = atlas.get(&key) else { continue; };` would skip the building entirely. We want it to fall back to frame 0 instead — but only for `can_be_occupied=true` Structures, so unrelated atlas misses still surface via the existing skip path (per memory `feedback_silent_render_failures`).

**Files:**
- Modify: [src/app_instances/shp.rs:151-159](../../src/app_instances/shp.rs#L151) — wrap the existing `atlas.get` lookup with a fallback retry.

**Pattern:** Local two-step lookup. No new helpers needed.

**Step 1: Replace lines 151-159**

Locate this block:

```rust
        let key: ShpSpriteKey = ShpSpriteKey {
            type_id: make_type_id.as_deref().unwrap_or(type_str).to_string(),
            facing: 0,
            frame: shp_frame,
            house_color: hc,
        };
        let Some(entry) = atlas.get(&key) else {
            continue;
        };
```

Replace with:

```rust
        let key: ShpSpriteKey = ShpSpriteKey {
            type_id: make_type_id.as_deref().unwrap_or(type_str).to_string(),
            facing: 0,
            frame: shp_frame,
            house_color: hc,
        };
        // Fallback: a CanBeOccupied building requesting frame 1/2/3 may miss the
        // atlas if its SHP has fewer than 4 frames. Retry with frame 0 so the
        // building still draws (Approach A in the design doc). Non-garrisonable
        // misses keep their existing skip path.
        let entry = match atlas.get(&key) {
            Some(e) => e,
            None if shp_frame != 0
                && entity.category == EntityCategory::Structure
                && state
                    .rules
                    .as_ref()
                    .and_then(|r| r.object(type_str))
                    .map(|o| o.can_be_occupied)
                    .unwrap_or(false) =>
            {
                let fallback_key = ShpSpriteKey {
                    type_id: make_type_id.as_deref().unwrap_or(type_str).to_string(),
                    facing: 0,
                    frame: 0,
                    house_color: hc,
                };
                match atlas.get(&fallback_key) {
                    Some(e) => e,
                    None => continue,
                }
            }
            None => continue,
        };
```

**Step 2: Verify build**

Run: `cargo check --lib`
Expected: PASS.

**Step 3: Commit**

```
render: fall back to frame 0 for garrisonables missing higher SHP frames
```

---

### Task 6: Build, lint, and run all tests

**Why:** Confirm the full crate compiles, clippy is clean for the new code, and no existing tests regress.

**Files:** None modified — verification only.

**Step 1: Full build**

Run: `cargo build`
Expected: PASS.

**Step 2: Clippy**

Run: `cargo clippy --lib -- -D warnings`
Expected: PASS. If clippy flags the new code (e.g., unused import, redundant clone), fix and re-run.

**Step 3: Test suite**

Run: `cargo test --lib`
Expected: All tests pass, including the 12 new `building_frame_index` tests from Task 2.

**Step 4: Commit (if any clippy fixes were needed)**

```
render: clippy fixes for garrison frame swap
```

---

### Task 7: In-game verification against gamemd.exe

**Why:** Resolve the deferred Open Question — does a healthy garrisoned civilian building actually show frame 2 in our renderer (matching player expectation), and does it match gamemd.exe behavior? This is the parity-critical moment.

**Files:** None modified — verification only.

**Step 1: Run the engine and load a skirmish**

Launch the game with a map containing visible civilian garrisonable buildings (e.g., any standard YR skirmish map with CABHUT/CALA-series buildings).

**Step 2: Verify each frame state**

| State | Action | Expected frame |
|---|---|---|
| Empty + healthy | Find a civilian garrisonable at full HP | 0 (default look) |
| Empty + red HP | Damage an empty civilian to <25% HP | 1 (damaged look) |
| Occupied + healthy | Garrison the building with infantry (Occupier=yes type, e.g. Conscript or GI) | **2 (lit-windows look)** |
| Occupied + red HP | Damage the garrisoned building to <25% HP | 1 (collapse rule — same as empty+red) |

**Step 3: Compare to gamemd.exe**

If you have gamemd.exe accessible, repeat the same scenarios in the original engine and compare side-by-side. Per [GARRISON_FRAME_SWAP_GHIDRA_REPORT.md §7 Open Question #1](../../../ra2-rust-game-docs/GARRISON_FRAME_SWAP_GHIDRA_REPORT.md), this resolves whether the BState gating in branch 4 actually fires for healthy civilians.

**Step 4: If healthy occupied does NOT show frame 2 in gamemd**

The binary must be gating the formula on damage tier in some way I missed. The fix is one line in Task 4 — wrap the helper call in:

```rust
let frame = if obj.map(|o| o.can_be_occupied).unwrap_or(false) {
    let ratio = if entity.health.max == 0 {
        1.0
    } else {
        entity.health.current as f32 / entity.health.max as f32
    };
    let cy = state.rules.as_ref().map(|r| r.general.condition_yellow).unwrap_or(0.5);
    if ratio <= cy {
        // ...existing helper call...
    } else {
        0
    }
} else {
    0
};
```

Document the finding back in `GARRISON_FRAME_SWAP_GHIDRA_REPORT.md` if the gate turns out to be needed.

**Step 5: Commit any post-observation fix**

```
render: <describe fix> based on gamemd.exe observation
```

---

## Sources & References

- **Design doc:** [docs/plans/2026-05-05-garrison-frame-swap-design.md](2026-05-05-garrison-frame-swap-design.md)
- **Primary research:** [ra2-rust-game-docs/GARRISON_FRAME_SWAP_GHIDRA_REPORT.md](../../../ra2-rust-game-docs/GARRISON_FRAME_SWAP_GHIDRA_REPORT.md)
- **Parent doc:** [ra2-rust-game-docs/BUILDINGCLASS_DRAWBODY_GHIDRA_REPORT.md](../../../ra2-rust-game-docs/BUILDINGCLASS_DRAWBODY_GHIDRA_REPORT.md) §3.2
- **Supporting research:** [ra2-rust-game-docs/TECHNOTYPECLASS_BASE_GHIDRA_REPORT.md](../../../ra2-rust-game-docs/TECHNOTYPECLASS_BASE_GHIDRA_REPORT.md) (TechLevel offset), [ra2-rust-game-docs/GARRISON_OCCUPANT_SYSTEM_GHIDRA_REPORT.md](../../../ra2-rust-game-docs/GARRISON_OCCUPANT_SYSTEM_GHIDRA_REPORT.md) (occupant count)
- **gamemd.exe addresses (kept here, not in code comments):**
  - `BuildingClass::GetCurrentFrame` @ `0x0043EF90`
  - `BuildingClass::GetOccupantCount` @ `0x004581F0` (returns `*(this+0x694)`)
  - `BuildingClass::Constructor` BState init @ `0x0043B770` (`OR EAX,-1; MOV [ESI+0x534], EAX`)
  - `RulesClass::ReadAudioVisual` @ `0x0066B34B-0x0066B372` (ConditionYellow/Red string xrefs)
  - `BuildingTypeClass+0x634` = `TechLevel`, `BuildingClass+0x520` = `Type`, `BuildingClass+0x534` = `CurrentAnimState (BState)`, `BuildingClass+0x694` = occupant count
  - `RulesClass+0x1700` = `ConditionYellow` (0.5), `RulesClass+0x1708` = `ConditionRed` (0.25)
- **INI keys:** `rulesmd.ini` per-building `CanBeOccupied=yes`, `TechLevel=`, `MaxNumberOccupants=`; `[AudioVisual]` `ConditionYellow=50%`, `ConditionRed=25%`
- **Repo patterns followed:** [src/app_ui_overlays.rs:766-769](../../src/app_ui_overlays.rs#L766) `health_pip_variant`, [src/app_instances/shp.rs:572](../../src/app_instances/shp.rs#L572) `resolve_infantry_shp_frame`
- **Related code:** [src/rules/object_type.rs:693](../../src/rules/object_type.rs#L693) (TechLevel parsing), [src/rules/object_type.rs:842](../../src/rules/object_type.rs#L842) (CanBeOccupied parsing), [src/rules/ruleset.rs:597-632](../../src/rules/ruleset.rs#L597-L632) (ConditionYellow/Red parsing), [src/sim/passenger.rs:213-217](../../src/sim/passenger.rs#L213-L217) (occupant count + integer ratio precedent)
