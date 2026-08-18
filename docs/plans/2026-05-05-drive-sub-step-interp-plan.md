# Drive Sub-Step Visual Interpolation Plan

> **For Claude:** Execute this plan task-by-task. Each task is self-contained.

**Goal:** Make moving vehicles drift smoothly between discrete drive-track steps by writing an interpolated lepton sub-cell position to `Position.sub_x/sub_y` every tick, matching gamemd.exe's `Process_Drive_Track` Phase 7 (RESIDUAL).

**Architecture:** Sim-side (Approach C). Extends [`drive_track::advance_drive_track`](../../src/sim/movement/drive_track.rs#L3599) to expose the next-step peek data needed for interp; adds an `interp_sub_step` helper; wires it into the mid-track normal path in [`movement_step::advance_lepton_position`](../../src/sim/movement/movement_step.rs#L238) at the `else` branch (movement_step.rs:285-291). No render changes — `Position.refresh_screen_coords` cascades to `screen_x/y`.

**Design Doc:** [docs/plans/2026-05-05-drive-sub-step-interp-design.md](2026-05-05-drive-sub-step-interp-design.md)

---

## Grounding Summary

- **ra2-rust-game-docs/:** [PROCESS_DRIVE_TRACK_DECOMPILATION.md §7](../../../ra2-rust-game-docs/PROCESS_DRIVE_TRACK_DECOMPILATION.md) covers Phase 7 RESIDUAL fully — interp factor `budget * (1/7)`, safety gate (`interp_cell ∈ {saved, full}` OR `budget > 3`), early exits, `EnterCell` apply path. Verified-from-binary.
- **Ghidra:** Confirmed `vtable+0x1B4 = 0x4DB810 = TechnoClass__Set_Coords_With_Cloak`, which writes the canonical pos via `ObjectClass::Set_Raw_Coords` — sub-step lepton position IS the sim position in the binary. Phase 7 entry point at `0x4b0f20` lines 696-793.
- **Repo pattern:** `Position.sub_x/sub_y: SimFixed` already supports sub-cell lepton precision. `Position.screen_x/y: f32` is `#[serde(skip, default)]` and updated via `refresh_screen_coords()` ([components.rs:51-60](../../src/sim/components.rs#L51-L60)) — already excluded from deterministic state hash. Existing `advance_drive_track` ([drive_track.rs:3599](../../src/sim/movement/drive_track.rs#L3599)) carries `state.residual` per-tick; we exercise it at sub-tick granularity.
- **INI keys:** None. The `1/7` factor and step cost `7` are hardcoded in the binary; no INI parsing needed.
- **Unknowns:** L12 — whether sub-step lepton pos shifts of ≤7 leptons have observable combat/vision effects in YR. Approach C makes this moot; we get the right answer either way.

## Key Technical Decisions

- **Sim-side write to `Position.sub_x/sub_y`** (not render-side smoothing): keeps combat/vision/fire-from-position consumers seeing the sub-step pos exactly as gamemd.exe does. — **Confidence:** high — **Source:** Ghidra `0x4DB810` writes canonical pos via `Set_Raw_Coords`.
- **Scope: mid-track normal path only.** Apply interp only when `advance_drive_track` exits with `!cell_jump && !chain_ready && !finished`. cell_jump and chain_ready paths use discrete-step coords as today (defer for parity follow-up). — **Confidence:** high — **Source:** matches binary's Phase 7 entry condition (loop exits on budget exhaustion, no chain success, not at track end).
- **Integer division `delta * residual / 7` with truncate-toward-zero rounding** matches binary's f64 `* 1/7` then `ftol`. — **Confidence:** high — **Source:** Rust's `/` on `i32` truncates toward zero; matches `ftol`.
- **L4 safety gate verbatim:** `interp_cell ∈ {saved_cell, full_step_cell}` OR `residual > 3` → use interp; else use full-step coords. — **Confidence:** high — **Source:** [PROCESS_DRIVE_TRACK_DECOMPILATION.md §7 lines 815-827].
- **Defer L7 (cloak re-eval per sub-step) and L9 (bridge sub-step transition).** Both are documented parity drifts in the design doc's ledger. — **Confidence:** high — **Source:** design doc Tiny-Detail Ledger.

## Open Questions

### Resolved During Planning

- **Where does Phase 7 fire in our caller flow?** The mid-track `else` branch at [movement_step.rs:285-291](../../src/sim/movement/movement_step.rs#L285-L291). Other branches (`cell_jump`, `chain_ready`, `finished`) bypass interp.
- **What is `saved_cell`?** Just `(position.rx, position.ry)` at tick entry. The mid-track-normal path means no cell change has occurred yet, so rx/ry are unchanged.
- **What is the next-step delta?** `transform_track_point(points[point_index + 1])` minus `transform_track_point(points[point_index])`. After post-loop, `state.point_index` is the last consumed point; the next step targets `point_index + 1`. The `head_offset_x/y` and `cell_offset_x/y` cancel in the subtraction.
- **Rounding direction:** Rust's `i32 / i32` truncates toward zero — matches `ftol`. No special handling needed.

### Deferred to Implementation

- None. All decisions are settled before tasks start.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | [src/sim/movement/drive_track.rs](../../src/sim/movement/drive_track.rs) | Extend `DriveTrackAdvance` with peek fields; populate them in `advance_drive_track`; add `interp_sub_step` helper. |
| Modify | [src/sim/movement/drive_track_tests.rs](../../src/sim/movement/drive_track_tests.rs) | Unit tests for peek and `interp_sub_step` math + safety gate. |
| Modify | [src/sim/movement/movement_step.rs](../../src/sim/movement/movement_step.rs) | Call `interp_sub_step` in the mid-track `else` branch; write through `Position` and `refresh_screen_coords`. |
| Modify | [src/sim/movement/movement_tests.rs](../../src/sim/movement/movement_tests.rs) | Update any test that pins per-tick discrete-step positions on moving vehicles. |

## Interface Changes

- **`DriveTrackAdvance` (public struct in `drive_track`)** gains three fields: `next_step_delta_x: i32`, `next_step_delta_y: i32`, `had_next_step: bool`. All existing consumers continue to work unchanged (struct is `Copy`-able, fields are additive). Direct consumer count: 1 (movement_step.rs:253). No breakage.
- **`interp_sub_step` (new pub(crate) function)** — pure, no I/O, fully testable. Takes saved sub-coords, saved cell, residual, and the peek delta; returns the interp result.

## Sim Checklist

- [x] All math uses `SimFixed` or `i32` integer arithmetic — no f32/f64 in game logic.
- [x] New state: none. `Position.sub_x/sub_y` is already part of the deterministic state hash.
- [x] No dependencies on render/ui/sidebar/audio/net.
- [x] Tick ordering: unchanged. Interp runs inside the existing ground-movement phase.
- [x] BTreeMap iteration order: unchanged.

## Risk Areas

- **Test pin churn.** Existing movement/combat tests that pin per-tick positions on moving vehicles will see `sub_x/sub_y` shift by sub-step amounts. Expected scope: `movement_tests.rs` and any combat test that exercises a moving Drive vehicle. One-time update; documented in Task 7.
- **L4 safety gate edge cases.** The `residual > 3` trust window is unusual; tests must cover it explicitly (Task 5).
- **Negative delta rounding.** Vehicles moving in negative-delta directions hit truncate-toward-zero rounding; verify in Task 4.
- **Cell-boundary interp_cell classification.** When `interp_cell` straddles a cell boundary in lepton space, `floor_div` must agree with the existing cell-jump computation at [drive_track.rs:3625-3631](../../src/sim/movement/drive_track.rs#L3625-L3631). Use the same `floor_div` helper (Task 3).

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|--------|------|----------------|--------------|
| Task 4 | Interp factor `delta * residual / 7` truncate-toward-zero | Every tick of every moving vehicle. Wrong rounding direction shifts the smoothness curve asymmetrically by direction. | Unit tests for residual ∈ {0..6}, including negative deltas. |
| Task 5 | L4 safety gate exact conditions | Without the gate, edge-case rounding can put `interp_cell` outside {saved, full}, producing a one-tick visual snap. | Unit tests for: interp lands in saved_cell; interp lands in full_step_cell; third-cell with residual > 3 (use interp anyway); third-cell with residual ≤ 3 (fall back to full). |
| Task 6 | Wiring scope: mid-track normal path only | Applying interp on cell_jump/chain_ready/finished paths would diverge from binary Phase 7 entry condition and risks double-applying offsets. | Code review: confirm `interp_sub_step` only called in the `else` branch at movement_step.rs:285-291. |
| Task 8 | End-to-end smoothness | The whole point. Without this, vehicles still snap. | Integration test: a Mirage advances N ticks; verify `screen_x` deltas are smaller than the discrete-step jump amount on intermediate ticks. |

---

## Tasks

### Task 1: Extend `DriveTrackAdvance` with peek fields

**Why:** Carry the next-step delta and a "has-next-step" flag out of `advance_drive_track` so the caller can run sub-step interp without re-walking the track tables.

**Files:**
- Modify: [src/sim/movement/drive_track.rs:3525-3544](../../src/sim/movement/drive_track.rs#L3525-L3544)

**Pattern:** Additive field extension on an existing public `Copy` struct. No breakage.

**Step 1: Edit the struct**

In [drive_track.rs:3525-3544](../../src/sim/movement/drive_track.rs#L3525-L3544), replace the existing `DriveTrackAdvance` struct definition with:

```rust
/// Result of advancing one tick through a drive track.
#[derive(Debug, Clone, Copy)]
pub struct DriveTrackAdvance {
    /// Sub-cell X position (transformed track point + head_offset + cell_offset).
    pub sub_x: SimFixed,
    /// Sub-cell Y position (transformed track point + head_offset + cell_offset).
    pub sub_y: SimFixed,
    /// Body facing at the current track point (transformed).
    pub facing: u8,
    /// True if the vehicle's position crossed into a different cell this tick.
    /// Detected by coordinate-based boundary checking — every step checks
    /// if the world position lands in a new cell.
    pub cell_jump: bool,
    /// True if the track reached the chain_index point. The caller should
    /// attempt to chain into the next track curve (check Can_Enter_Cell on
    /// the next-next cell, select new track if passable).
    pub chain_ready: bool,
    /// True if the track has been fully traversed.
    pub finished: bool,
    /// Lepton-space delta from the just-consumed point to the next-to-consume
    /// point, transformed by the active flags. Zero when no next step exists
    /// (track end / sentinel hit). Used by sub-step interp to scale fractional
    /// progress from the residual budget.
    pub next_step_delta_x: i32,
    /// See `next_step_delta_x`.
    pub next_step_delta_y: i32,
    /// True iff a valid next step exists (not at last_index, not a sentinel).
    /// Sub-step interp is only applied when this is true.
    pub had_next_step: bool,
}
```

**Step 2: Verify compile**

Run: `cargo check -p ra2-rust-game`
Expected: compile errors at the two `DriveTrackAdvance { ... }` literal sites in `advance_drive_track` (lines 3660-3667 and 3669-3676) — those construct the struct without the new fields. We fix those in Task 2.

**Step 3: Commit**

`drive_track: extend DriveTrackAdvance with next-step peek fields`

---

### Task 2: Compute next-step peek inside `advance_drive_track`

**Why:** Populate the new peek fields on every return path — both the normal (idx-in-range) path and the off-end fallback path. Match the existing `transform_track_point` invocation pattern.

**Files:**
- Modify: [src/sim/movement/drive_track.rs:3654-3677](../../src/sim/movement/drive_track.rs#L3654-L3677)

**Pattern:** Mirrors the existing `transform_track_point` use at line 3659.

**Step 1: Update the normal-return arm**

Replace [drive_track.rs:3656-3667](../../src/sim/movement/drive_track.rs#L3656-L3667):

```rust
    // Read position and facing from current track point, applying
    // transform flags and head/cell offsets.
    let idx = state.point_index as usize;
    if idx < points.len() {
        let pt = &points[idx];
        let (tx, ty, tf) = transform_track_point(pt.x, pt.y, pt.facing, state.transform_flags);

        // Peek the next-to-consume point for sub-step interp.
        // Skip if at last_index (no next step) or if the next point is the
        // (0, 0) sentinel at a non-zero index (matches L6 in design ledger).
        let next_idx = idx + 1;
        let (next_dx, next_dy, has_next) = if next_idx < points.len()
            && (next_idx as u16) <= last_index
        {
            let npt = &points[next_idx];
            let (ntx, nty, _) = transform_track_point(npt.x, npt.y, npt.facing, state.transform_flags);
            let is_sentinel = npt.x == 0 && npt.y == 0 && next_idx != 0;
            if is_sentinel {
                (0, 0, false)
            } else {
                ((ntx as i32) - (tx as i32), (nty as i32) - (ty as i32), true)
            }
        } else {
            (0, 0, false)
        };

        DriveTrackAdvance {
            sub_x: SimFixed::from_num(state.head_offset_x + tx as i32 + state.cell_offset_x),
            sub_y: SimFixed::from_num(state.head_offset_y + ty as i32 + state.cell_offset_y),
            facing: tf,
            cell_jump,
            chain_ready,
            finished,
            next_step_delta_x: next_dx,
            next_step_delta_y: next_dy,
            had_next_step: has_next,
        }
    } else {
```

**Step 2: Update the fallback arm**

Replace the `else` branch at [drive_track.rs:3668-3677](../../src/sim/movement/drive_track.rs#L3668-L3677):

```rust
    } else {
        DriveTrackAdvance {
            sub_x: SimFixed::from_num(128),
            sub_y: SimFixed::from_num(128),
            facing: 0,
            cell_jump: false,
            chain_ready: false,
            finished: true,
            next_step_delta_x: 0,
            next_step_delta_y: 0,
            had_next_step: false,
        }
    }
```

**Step 3: Verify compile**

Run: `cargo check -p ra2-rust-game`
Expected: clean compile.

Run: `cargo test -p ra2-rust-game drive_track --no-run`
Expected: clean compile of existing drive_track tests.

**Step 4: Run existing drive_track tests**

Run: `cargo test -p ra2-rust-game drive_track`
Expected: all existing tests pass — peek fields are additive, no behavior change yet.

**Step 5: Commit**

`drive_track: populate next-step peek fields on every return path`

---

### Task 3: Add `interp_sub_step` helper

**Why:** Encapsulate the L2 (interp factor), L4 (safety gate), L5/L6 (early exits) logic in a single pure function the caller can invoke after `advance_drive_track`.

**Files:**
- Modify: [src/sim/movement/drive_track.rs](../../src/sim/movement/drive_track.rs) — add new helper after `advance_drive_track`.

**Pattern:** Pure function, integer arithmetic only, no `state` mutation. Mirrors the helper-function placement of `floor_div` and `transform_track_point`.

**Step 1: Add `InterpSubStepResult` and the helper**

After the closing `}` of `advance_drive_track` at [drive_track.rs:3678](../../src/sim/movement/drive_track.rs#L3678) (and before the `#[cfg(test)]` block at line 3684), insert:

```rust
/// Sub-step interp constants.
const TRACK_STEP_DENOM: i32 = 7;
/// Above-this residual triggers L4's "trust window" (use interp even if cell
/// classification looks unexpected).
const INTERP_TRUST_BUDGET: i32 = 3;

/// Output of `interp_sub_step`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct InterpSubStepResult {
    /// Sub-cell X to write to `Position.sub_x`. Always within the saved cell's
    /// or the full-step cell's [0, 256) lepton range.
    pub sub_x: SimFixed,
    /// Sub-cell Y to write to `Position.sub_y`.
    pub sub_y: SimFixed,
}

/// Compute a sub-step interpolated position from the residual budget and the
/// next-step peek. Returns `None` when no interp should be applied this tick
/// (residual < 1, no next step, or any L5/L6 early-exit condition).
///
/// `saved_sub_x/saved_sub_y` are the lepton sub-cell coords AFTER the discrete
/// step loop ran (the result of `advance_drive_track`). The cell anchor
/// (`Position.rx/ry`) is implicit — the L4 safety gate uses cell-relative
/// offsets, so the absolute cell coords are not needed here. The mid-track
/// normal path guarantees rx/ry are unchanged from tick entry.
/// `next_delta_x/next_delta_y` are from `DriveTrackAdvance.next_step_delta_*`.
/// `residual` is `state.residual` (range `0..=6` after a normal step-loop exit).
pub(crate) fn interp_sub_step(
    saved_sub_x: SimFixed,
    saved_sub_y: SimFixed,
    next_delta_x: i32,
    next_delta_y: i32,
    residual: i32,
    had_next_step: bool,
) -> Option<InterpSubStepResult> {
    // L5: no interp when residual is zero (vehicle made no fractional progress
    // this tick — either it consumed exactly 7-multiples of budget, or it had
    // none to begin with).
    if residual < 1 {
        return None;
    }
    // L5/L6: no interp without a valid next step (track end or sentinel).
    if !had_next_step {
        return None;
    }

    // L2: interp offset = next_delta * residual / 7 (truncate toward zero).
    let interp_dx: i32 = next_delta_x * residual / TRACK_STEP_DENOM;
    let interp_dy: i32 = next_delta_y * residual / TRACK_STEP_DENOM;

    // Full-step offset (what the next step would produce at residual = 7).
    let full_dx: i32 = next_delta_x;
    let full_dy: i32 = next_delta_y;

    // Convert saved sub-coords to absolute lepton-space (saved_cell-origin).
    let saved_lx: i32 = saved_sub_x.to_num::<i32>();
    let saved_ly: i32 = saved_sub_y.to_num::<i32>();

    // L4 cell membership: classify which cell the interp/full positions land in.
    // We only care about the cell-relative offset; the result is added to
    // (saved_rx, saved_ry) to get the absolute cell.
    let interp_cell_dx: i32 = floor_div(saved_lx + interp_dx, 256);
    let interp_cell_dy: i32 = floor_div(saved_ly + interp_dy, 256);
    let full_cell_dx: i32 = floor_div(saved_lx + full_dx, 256);
    let full_cell_dy: i32 = floor_div(saved_ly + full_dy, 256);

    let in_saved = interp_cell_dx == 0 && interp_cell_dy == 0;
    let in_full = interp_cell_dx == full_cell_dx && interp_cell_dy == full_cell_dy;

    // L4 gate: use interp if it lands in saved or full cell, OR residual > 3
    // (the trust window — past the step midpoint, trust the interp even if
    // cell classification looks off).
    let use_interp = in_saved || in_full || residual > INTERP_TRUST_BUDGET;

    if use_interp {
        Some(InterpSubStepResult {
            sub_x: SimFixed::from_num(saved_lx + interp_dx),
            sub_y: SimFixed::from_num(saved_ly + interp_dy),
        })
    } else {
        // L4 fallback: use full-step coords.
        Some(InterpSubStepResult {
            sub_x: SimFixed::from_num(saved_lx + full_dx),
            sub_y: SimFixed::from_num(saved_ly + full_dy),
        })
    }
}
```

**Step 2: Verify compile**

Run: `cargo check -p ra2-rust-game`
Expected: clean compile.

**Step 3: Commit**

`drive_track: add interp_sub_step helper with L4 safety gate`

---

### Task 4: Unit tests for `interp_sub_step` math (L2 / L11)

**Why:** Pin the rounding direction for the `delta * residual / 7` integer division across positive and negative deltas, and across all valid residual values.

**Files:**
- Modify: [src/sim/movement/drive_track_tests.rs](../../src/sim/movement/drive_track_tests.rs) — append to the existing test module.

**Pattern:** Plain `#[test]` functions, mirrors existing tests at [drive_track_tests.rs:351+](../../src/sim/movement/drive_track_tests.rs#L351).

**Step 1: Append tests at the end of the file**

```rust
// ---------------------------------------------------------------------------
// interp_sub_step tests
// ---------------------------------------------------------------------------

#[test]
fn interp_sub_step_residual_zero_returns_none() {
    let result = interp_sub_step(
        SimFixed::from_num(128),
        SimFixed::from_num(128),
        14,
        0,
        0,
        true,
    );
    assert_eq!(result, None, "residual=0 must yield no interp");
}

#[test]
fn interp_sub_step_no_next_step_returns_none() {
    let result = interp_sub_step(
        SimFixed::from_num(128),
        SimFixed::from_num(128),
        14,
        0,
        3,
        false,
    );
    assert_eq!(result, None, "had_next_step=false must yield no interp");
}

#[test]
fn interp_sub_step_fraction_at_residual_1() {
    // delta=14, residual=1 → 14 * 1 / 7 = 2.
    let result = interp_sub_step(
        SimFixed::from_num(100),
        SimFixed::from_num(100),
        14,
        0,
        1,
        true,
    )
    .expect("interp should apply");
    assert_eq!(result.sub_x, SimFixed::from_num(102), "saved 100 + 14*1/7=2 → 102");
    assert_eq!(result.sub_y, SimFixed::from_num(100));
}

#[test]
fn interp_sub_step_fraction_at_residual_6() {
    // delta=14, residual=6 → 14 * 6 / 7 = 12.
    let result = interp_sub_step(
        SimFixed::from_num(100),
        SimFixed::from_num(100),
        14,
        0,
        6,
        true,
    )
    .expect("interp should apply");
    assert_eq!(result.sub_x, SimFixed::from_num(112), "saved 100 + 14*6/7=12 → 112");
}

#[test]
fn interp_sub_step_negative_delta_truncates_toward_zero() {
    // delta=-15, residual=3 → -15 * 3 / 7 = -45 / 7 = -6 (truncated from -6.43).
    let result = interp_sub_step(
        SimFixed::from_num(200),
        SimFixed::from_num(100),
        -15,
        0,
        3,
        true,
    )
    .expect("interp should apply");
    assert_eq!(
        result.sub_x,
        SimFixed::from_num(194),
        "saved 200 + (-15)*3/7=-6 → 194 (truncate toward zero on negative)"
    );
}

#[test]
fn interp_sub_step_diagonal_delta() {
    // dx=14, dy=-7, residual=4 → dx*4/7=8, dy*4/7=-4.
    let result = interp_sub_step(
        SimFixed::from_num(100),
        SimFixed::from_num(100),
        14,
        -7,
        4,
        true,
    )
    .expect("interp should apply");
    assert_eq!(result.sub_x, SimFixed::from_num(108));
    assert_eq!(result.sub_y, SimFixed::from_num(96));
}

#[test]
fn interp_sub_step_all_residual_values_monotonic() {
    // For positive delta, sub_x must increase monotonically with residual.
    let mut last = SimFixed::from_num(100);
    for r in 1..=6 {
        let result = interp_sub_step(
            SimFixed::from_num(100),
            SimFixed::from_num(100),
            14,
            0,
            r,
            true,
        )
        .expect("interp should apply");
        assert!(
            result.sub_x > last,
            "residual {} produced sub_x {:?} not greater than previous {:?}",
            r,
            result.sub_x,
            last
        );
        last = result.sub_x;
    }
}
```

**Step 2: Run tests**

Run: `cargo test -p ra2-rust-game interp_sub_step`
Expected: all 7 new tests PASS.

**Step 3: Commit**

`drive_track: tests for interp_sub_step math (L2/L11 rounding)`

---

### Task 5: Unit tests for L4 safety gate

**Why:** Pin the cell-classification logic and the `residual > 3` trust window — the trickiest piece of Phase 7 to get right.

**Files:**
- Modify: [src/sim/movement/drive_track_tests.rs](../../src/sim/movement/drive_track_tests.rs) — append more tests.

**Pattern:** Same as Task 4. Construct inputs that force interp_cell into specific cells.

**Step 1: Append tests**

```rust
#[test]
fn interp_sub_step_lands_in_saved_cell() {
    // saved=(100, 100), delta=(14, 0), residual=2 → interp=(104, 100).
    // floor_div(100+4, 256) = 0, floor_div(100, 256) = 0. interp_cell == saved.
    let result = interp_sub_step(
        SimFixed::from_num(100),
        SimFixed::from_num(100),
        14,
        0,
        2,
        true,
    )
    .expect("interp should apply");
    assert_eq!(result.sub_x, SimFixed::from_num(104));
}

#[test]
fn interp_sub_step_lands_in_full_step_cell() {
    // saved=(250, 100), full delta=(14, 0). residual=6 → interp_dx = 12.
    // saved_lx + interp_dx = 262 → cell offset (1, 0). saved_lx + full_dx = 264 → cell offset (1, 0).
    // interp_cell == full_cell, so use interp.
    let result = interp_sub_step(
        SimFixed::from_num(250),
        SimFixed::from_num(100),
        14,
        0,
        6,
        true,
    )
    .expect("interp should apply");
    // 250 + 14*6/7 = 250 + 12 = 262.
    assert_eq!(result.sub_x, SimFixed::from_num(262));
}

#[test]
fn interp_sub_step_third_cell_with_high_residual_uses_interp() {
    // Third-cell construction: saved=(0, 0), delta=(770, 0), residual=4.
    // interp_dx = 770*4/7 = 440. saved+interp = 440 → cell offset 1.
    // full_dx = 770. saved+full = 770 → cell offset 3 (770 = 3*256+2).
    // saved cell offset 0, interp 1, full 3. Third-cell case.
    // residual=4 > 3 → use interp despite third-cell classification.
    let result = interp_sub_step(
        SimFixed::from_num(0),
        SimFixed::from_num(0),
        770,
        0,
        4,
        true,
    )
    .expect("interp should apply");
    // residual > 3 → use interp: 0 + 440 = 440.
    assert_eq!(result.sub_x, SimFixed::from_num(440));
}

#[test]
fn interp_sub_step_third_cell_with_low_residual_falls_back() {
    // saved=(0, 0), delta=(2000, 0), residual=2.
    // interp_dx = 2000*2/7 = 571. saved+interp = 571 → cell offset 2.
    // full_dx = 2000. saved+full = 2000 → cell offset 7.
    // saved 0, interp 2, full 7. Third-cell case.
    // residual=2 ≤ 3 → fall back to full-step coords.
    let result = interp_sub_step(
        SimFixed::from_num(0),
        SimFixed::from_num(0),
        2000,
        0,
        2,
        true,
    )
    .expect("interp should apply (fallback path)");
    // L4 fallback: use full-step coords.
    assert_eq!(
        result.sub_x,
        SimFixed::from_num(2000),
        "low residual + third-cell interp must fall back to full-step coords"
    );
}

#[test]
fn interp_sub_step_residual_threshold_is_strict_greater() {
    // residual = 3 must NOT trigger the trust window (gate is > 3, not >= 3).
    // saved=(0, 0), delta=(2000, 0), residual=3.
    // interp_dx = 2000*3/7 = 857. saved+interp = 857 → cell offset 3.
    // full = 2000 → cell offset 7. Third-cell case.
    // residual=3 NOT > 3 → fall back to full.
    let result = interp_sub_step(
        SimFixed::from_num(0),
        SimFixed::from_num(0),
        2000,
        0,
        3,
        true,
    )
    .expect("interp should apply (fallback path)");
    assert_eq!(
        result.sub_x,
        SimFixed::from_num(2000),
        "residual=3 with third-cell interp must fall back (gate is > 3, not >= 3)"
    );
}

#[test]
fn interp_sub_step_residual_4_triggers_trust_window() {
    // Same construction with residual=4 — should now USE interp.
    // interp_dx = 2000*4/7 = 1142. saved+interp = 1142 → cell offset 4.
    // full = 2000 → cell offset 7. Third-cell, residual=4 > 3 → trust window triggered.
    let result = interp_sub_step(
        SimFixed::from_num(0),
        SimFixed::from_num(0),
        2000,
        0,
        4,
        true,
    )
    .expect("interp should apply");
    assert_eq!(
        result.sub_x,
        SimFixed::from_num(1142),
        "residual=4 trust window: use interp despite third-cell"
    );
}
```

**Step 2: Run tests**

Run: `cargo test -p ra2-rust-game interp_sub_step`
Expected: all 13 interp tests (Tasks 4 + 5) PASS.

**Step 3: Commit**

`drive_track: tests for interp_sub_step L4 safety gate (cell + trust window)`

---

### Task 6: Wire `interp_sub_step` into `advance_lepton_position`

**Why:** Apply the interp result to `Position.sub_x/sub_y` on the mid-track normal path. cell_jump, chain_ready, and finished paths are unchanged.

**Files:**
- Modify: [src/sim/movement/movement_step.rs:285-291](../../src/sim/movement/movement_step.rs#L285-L291)

**Pattern:** Mirrors the existing position-write pattern. `refresh_screen_coords()` at the same call site as today.

**Step 1: Update the mid-track else branch**

Replace [movement_step.rs:285-291](../../src/sim/movement/movement_step.rs#L285-L291):

```rust
        } else {
            // Mid-track, no events — apply discrete-step pos, then layer
            // sub-step interp on top using the residual budget. The interp
            // helper enforces the L4 cell-validity safety gate; cell occupancy
            // never changes mid-step (rx/ry unchanged on this path).
            position.sub_x = advance.sub_x;
            position.sub_y = advance.sub_y;
            if let Some(interp) = drive_track::interp_sub_step(
                advance.sub_x,
                advance.sub_y,
                advance.next_step_delta_x,
                advance.next_step_delta_y,
                track_state.residual,
                advance.had_next_step,
            ) {
                position.sub_x = interp.sub_x;
                position.sub_y = interp.sub_y;
            }
            position.refresh_screen_coords();
            return AdvanceResult::DriveTrackActive;
        }
```

**Step 2: Verify compile**

Run: `cargo check -p ra2-rust-game`
Expected: clean compile. (`drive_track::interp_sub_step` is `pub(crate)` — accessible across modules within the crate.)

**Step 3: Run existing drive_track integration tests**

Run: `cargo test -p ra2-rust-game advance_drive_track`
Expected: tests still PASS — the existing `drive_track_tests.rs` exercises `advance_drive_track` directly, not the wired path through `movement_step`.

**Step 4: Commit**

`movement_step: apply interp_sub_step on mid-track normal path`

---

### Task 7: Update existing tests that pin per-tick discrete-step positions

**Why:** Tests that previously asserted `position.sub_x == <discrete-step value>` after a single tick will now see sub-step-interpolated values. Update expectations.

**Files:**
- Modify: [src/sim/movement/movement_tests.rs](../../src/sim/movement/movement_tests.rs) — update affected tests.
- (Other test files only if they exercise a moving Drive vehicle and pin sub-cell coords.)

**Pattern:** No new pattern. Update assertions to either (a) accept a small tolerance band, or (b) advance the simulation past the residual-clearing tick boundary before asserting.

**Step 1: Identify affected tests**

Run: `cargo test -p ra2-rust-game movement` (no filter — runs all movement tests).
Expected: any failures point to tests that pin per-tick positions on Drive vehicles. If the suite passes as-is, this task is no-op (continue to Step 4).

**Step 2: Inspect each failure and pick the appropriate fix**

For each failing test, decide:
- If the test was asserting "after N ticks, vehicle is at X" — the value may have shifted by ≤ a few leptons. Update the expected value to match the new (correct) sub-step pos.
- If the test was asserting per-tick monotonicity / cell-crossing semantics — those are still correct; update wording/comments only.
- If the test was using a discrete-step expectation as a proxy for "moved this far" — change it to a tolerance-based assertion (`assert!((actual - expected).abs() < 8)`).

Be conservative: don't loosen tests beyond what's necessary. If a test legitimately needs per-tick discrete-step semantics, advance the simulation by enough ticks for residual to clear (residual eventually multiples back to 0 when speed × dt is a multiple of 7).

**Step 3: Re-run the movement test suite**

Run: `cargo test -p ra2-rust-game movement`
Expected: PASS.

**Step 4: Run the full test suite to catch any combat / passenger / chrono / miner test regressions**

Run: `cargo test -p ra2-rust-game`
Expected: PASS. Investigate any regressions; same fix-strategy as Step 2.

**Step 5: Commit**

`movement_tests: update per-tick position expectations for sub-step interp`

If no tests required updates, skip this commit and note in the next task that the test suite was clean.

---

### Task 8: End-to-end smoothness integration test

**Why:** Verify the whole pipeline (advance_drive_track → interp_sub_step → Position write → screen_x recompute) produces sub-7-lepton incremental motion, not 7-lepton snaps.

**Files:**
- Modify: [src/sim/movement/drive_track_tests.rs](../../src/sim/movement/drive_track_tests.rs) — append integration-style test.

**Pattern:** Construct a `DriveTrackState`, run `advance_drive_track` + `interp_sub_step` for many ticks, assert that the per-tick lepton delta is bounded.

**Step 1: Append the test**

```rust
#[test]
fn end_to_end_sub_step_smoothness_no_seven_lepton_snaps() {
    // Pick a track that produces non-trivial deltas (track 1 = straight ahead from
    // the existing test at line 354). Use a speed that produces partial steps per
    // tick (5 leptons/tick = below the 7-budget step cost, so residual is always
    // present for interp).
    let mut state = drive_track::begin_drive_track(1, 0, 0, 1).expect("track 1 exists");
    let dt = SimFixed::from_num_ratio(1, 15); // ~66ms
    let speed = SimFixed::from_num(75); // 75 * 0.066 ≈ 5 leptons/tick

    // Track per-tick lepton delta (in saved+interp space). We expect every
    // delta to be ~5 leptons — never 7+ (no full-step snaps), never 0 (no
    // stalls after first tick).
    let mut prev_x: Option<SimFixed> = None;
    let mut prev_y: Option<SimFixed> = None;
    let mut max_delta: i32 = 0;
    let mut zero_delta_ticks: i32 = 0;

    for _tick in 0..30 {
        let advance = drive_track::advance_drive_track(&mut state, speed, dt);
        if advance.finished {
            break;
        }

        let mut sub_x = advance.sub_x;
        let mut sub_y = advance.sub_y;
        if let Some(interp) = drive_track::interp_sub_step(
            sub_x,
            sub_y,
            advance.next_step_delta_x,
            advance.next_step_delta_y,
            state.residual,
            advance.had_next_step,
        ) {
            sub_x = interp.sub_x;
            sub_y = interp.sub_y;
        }

        if let (Some(px), Some(py)) = (prev_x, prev_y) {
            let dx = (sub_x - px).to_num::<i32>().abs();
            let dy = (sub_y - py).to_num::<i32>().abs();
            let d = dx.max(dy);
            max_delta = max_delta.max(d);
            if d == 0 {
                zero_delta_ticks += 1;
            }
        }
        prev_x = Some(sub_x);
        prev_y = Some(sub_y);
    }

    assert!(
        max_delta < 7,
        "max per-tick delta {} should be below TRACK_STEP_COST (7) — \
         a delta of 7+ indicates a full-step snap, meaning interp didn't fire",
        max_delta
    );
    assert!(
        zero_delta_ticks <= 1,
        "found {} ticks with zero per-tick delta — vehicle is stalling between snaps \
         instead of drifting smoothly",
        zero_delta_ticks
    );
}
```

**Step 2: Run the test**

Run: `cargo test -p ra2-rust-game end_to_end_sub_step_smoothness`
Expected: PASS.

**Step 3: Commit**

`drive_track: end-to-end smoothness test for sub-step interp`

---

### Task 9: Verification against gamemd.exe

**Why:** Confirm the implementation matches Phase 7 behavior. State-hash divergence localization may flag sub-step churn — that's expected.

**Verify:**
- **Behavior:** Run the engine against a stock skirmish map. Watch a Mirage Tank or IFV in motion. Compared to pre-change, vehicles should drift smoothly between cell positions instead of snapping. Compared to gamemd.exe in the same map, motion should look effectively identical.
- **Determinism:** Run two identical simulations and verify identical state hashes after a long run with moving vehicles. Confirms the fixed-point interp is deterministic.
- **Test suite:** `cargo test -p ra2-rust-game` — full suite green.
- **Clippy:** `cargo clippy -p ra2-rust-game -- -D warnings` — clean.

**Source:** [PROCESS_DRIVE_TRACK_DECOMPILATION.md §7](../../../ra2-rust-game-docs/PROCESS_DRIVE_TRACK_DECOMPILATION.md). Phase 7 entry at gamemd.exe `0x4b0f20`.

**No commit** — this is a verification gate. If anything fails, drop back into the relevant task and fix before proceeding.

---

## Sources & References

- **Design doc:** [docs/plans/2026-05-05-drive-sub-step-interp-design.md](2026-05-05-drive-sub-step-interp-design.md)
- **Ghidra reports:** [PROCESS_DRIVE_TRACK_DECOMPILATION.md §7](../../../ra2-rust-game-docs/PROCESS_DRIVE_TRACK_DECOMPILATION.md) (Phase 7 RESIDUAL).
- **gamemd.exe addresses:**
  - `0x4b0f20` — `Process_Drive_Track` (Phase 7 entry point)
  - `0x4DB810` — `TechnoClass__Set_Coords_With_Cloak` (vtable+0x1B4 = EnterCell)
  - `0x7E7FA8` — `1/7` constant used by Phase 7
- **INI keys:** None.
- **Related code:**
  - [src/sim/movement/drive_track.rs:3525-3678](../../src/sim/movement/drive_track.rs#L3525-L3678) — DriveTrackAdvance struct + advance_drive_track function
  - [src/sim/movement/movement_step.rs:238-344](../../src/sim/movement/movement_step.rs#L238-L344) — advance_lepton_position
  - [src/sim/components.rs:31-60](../../src/sim/components.rs#L31-L60) — Position struct + refresh_screen_coords
- **Prior gap scan:** [docs/gap-scans/2026-05-05c-gap-scan-drive_track-deep.md](../../docs/gap-scans/2026-05-05c-gap-scan-drive_track-deep.md) (D3 §4 + D5 §2 — origin of this work).
