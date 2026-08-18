# A* Uniform Edge Cost Implementation Plan

> **For Claude:** Execute this plan task-by-task. Each task is self-contained.

**Goal:** Replace octile A* (cardinal=10, diagonal=14, octile heuristic) with gamemd.exe's uniform-cost / Euclidean-heuristic model so the pathfinder returns the same paths the original engine does.

**Architecture:** Single-file change in [src/sim/pathfinding/core.rs](src/sim/pathfinding/core.rs) (constants + heuristic + cost selector) plus matching test updates in [src/sim/pathfinding/core_tests.rs](src/sim/pathfinding/core_tests.rs). All existing public APIs (`find_path`, `find_path_with_costs`, `find_path_with_costs_corridor`, `find_layered_path`, `astar_search`) keep their signatures. No other modules touched.

**Design Doc:** [docs/plans/2026-05-07-astar-uniform-cost-design.md](docs/plans/2026-05-07-astar-uniform-cost-design.md)

---

## Grounding Summary

- **Docs:** `ra2-rust-game-docs/PATHFINDING_CELL_ENTRY_VERIFICATION_REPORT.md` §1.1, §1.3, §1.5, §6.2 — base cost table at `0x0081870c`, cliff multiplier 4×, direction tiebreaker table at `0x0081872c`, "no diagonal multiplier" finding. All HIGH confidence per the report.
- **Ghidra (verified live this session):**
  - `AStar_compute_edge_cost @ 0x00429830` — function takes `(this, cell, bridge_flag, can_enter_code, pathfinder)`. **No direction parameter.** Confirms uniform edge cost across 8 compass directions.
  - `AStar_create_node @ 0x0042a460` — final lines compute `Sqrt_Approx(dx² + dy²)` and add it to `g_cost` to populate `node[+8]` (the f_cost field used in heap compares). **Heuristic is pure Euclidean.**
  - `AStar_main_loop @ 0x00429a90` — composes step cost as `edge_cost × cost_multiplier + tiebreaker[direction]`, accumulates into g; passes that to `AStar_create_node` as the edge increment. Tiebreaker accumulates into g_cost.
- **Repo pattern:** existing constants/heuristic/cost-selector layout in [src/sim/pathfinding/core.rs](src/sim/pathfinding/core.rs) is followed; this is a value/scalar swap, not a structural change. Tests live in `core_tests.rs` per the existing split-out pattern.
- **INI keys:** none. A* base costs are hard-coded `.rdata` floats in the binary — not driven by `rules(md).ini` / `art(md).ini`.
- **Still unknown after grounding (deferred, NOT in this plan):**
  - "Code 4 = 60.0" semantics — doc §1.1 says OccupiedFriendly, §6.3 says Wall/gate. Needs `/re-investigate` before a separate fix.
  - Bridge flanking diagonal multipliers (1.0/2.0/10.0) per doc §1.4 — distinct system, separate item.
  - `PathfinderClass+0x04` global edge-cost multiplier — separate item.

## Key Technical Decisions

- **STEP_COST = 1000** (replaces both `CARDINAL_COST=10` and `DIAGONAL_COST=14`) — **Confidence: high**
  - **Source:** doc §1.1 (binary base 1.0) + design doc Chosen Approach (×1000 scalar so existing `DIR_TIEBREAK = [1..=8]` lands at exactly 0.001..0.008 of base, matching binary's `0x0081872c` ratio).
- **Euclidean heuristic via integer isqrt** — **Confidence: high**
  - **Source:** Ghidra `AStar_create_node @ 0x0042a460` (verified live this session — `Sqrt_Approx(dx² + dy²)` literal).
  - Implementation: `((dx² + dy²) × 1_000_000).isqrt() as i32` keeps full precision; `u64` intermediate avoids overflow on 512×512 maps.
- **Heuristic kept inadmissible under uniform cost** — **Confidence: high**
  - **Source:** binary itself does this; matching deliberately for parity. Path-shape characteristic of the original engine.
- **`DIR_TIEBREAK` array values [1..=8] unchanged** — **Confidence: high**
  - **Source:** doc §1.5; ratio relative to base now matches binary directly (1/1000..8/1000 ≈ 0.001..0.008).
- **`CLIFF_COST_MULTIPLIER`, code 2/5/6 multipliers unchanged** — **Confidence: high**
  - **Source:** they multiply `step_cost`, so the scale change passes through proportionally.
- **`u64::isqrt` from std (stable since Rust 1.84)** — **Confidence: high**
  - **Source:** verified `rust-toolchain.toml` channel = "stable", edition = "2024" → guaranteed 1.84+.

No low-confidence decisions. `/review-plan` should still spot-check the Ghidra claims (addresses, function names) and the integer-overflow analysis.

## Open Questions

### Resolved During Planning

- **Q: What heuristic does the binary actually use?** → Euclidean. Verified live in `AStar_create_node @ 0x0042a460` this session.
- **Q: Is `u64::isqrt` available on our toolchain?** → Yes. Stable channel, edition 2024, `u64::isqrt` stable since 1.84.
- **Q: Does the f_cost float→int store in `AStar_create_node` indicate integer or float comparison?** → Heap compares are `*(float*)`; the `(int)` cast in decompilation is Ghidra's type confusion. Field is float in binary; we use i32 with our 1000× scale and the comparison semantics still hold.

### Deferred to Implementation

- **Will any currently-passing pathfinding test fail purely due to changed path geometry (not because of new constants)?** → Most existing tests assert path *existence* and *first/last cells*, not specific intermediate cells. Will know after Task 4 runs the suite. Mitigation noted in Task 4.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | `src/sim/pathfinding/core.rs` | Constants, cost selector, heuristic |
| Modify | `src/sim/pathfinding/core_tests.rs` | Update 3 heuristic tests; add 3 parity tests |

## Interface Changes

None. All public functions (`find_path`, `find_path_with_costs`, `find_path_with_costs_corridor`, `find_layered_path`, `astar_search`) keep their signatures. Internal renames:
- `octile_heuristic` → `euclidean_heuristic` (private)
- `CARDINAL_COST` + `DIAGONAL_COST` → `STEP_COST` (private)

## Sim Checklist

- [x] All math uses integer types — `i32` for costs, `u64` for the heuristic intermediate. No f32/f64.
- [x] No new state added; deterministic state hash unaffected (same fields, different values).
- [x] No dependencies on render/ui/sidebar/audio/net.
- [x] Tick ordering unaffected — pathfinder is invoked from movement code, no order change.
- [x] BTreeMap iteration order unaffected — A* operates on PathGrid arrays, not EntityStore.

## Risk Areas

- **Replays diverge.** Old replays will desync against new code. Acceptable per design.
- **Existing tests asserting specific path cells.** A handful of tests check intermediate cells (e.g. corner-cutting test, water diagonal test). With uniform cost, geometry of returned paths can shift even when start/goal/length are unchanged. Task 4 catches and addresses any breakage.
- **Integer overflow in heuristic intermediate.** `(dx² + dy²) × 1_000_000` for 512×512 maps maxes at ~5.22×10¹¹, fits u64 trivially. `isqrt` result fits i32. Verified in design doc.
- **AStarNode::cmp tiebreak ordering.** Preserved as-is; only the values feeding `f_cost` change.

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|--------|------|----------------|--------------|
| Task 1 | `STEP_COST = 1000` (not 10) — preserves DIR_TIEBREAK ratio at exactly 0.001..0.008 of base | Tiebreaker selects path under f-cost ties; wrong ratio = different path than binary | Inspection vs binary `0x0081872c` table |
| Task 2 | Heuristic = `Sqrt_Approx(dx² + dy²)` — Euclidean, inadmissible under uniform cost | Binary uses Euclidean intentionally; switching to Chebyshev would give optimal-but-different paths | Ghidra `AStar_create_node @ 0x0042a460` |
| Task 2 | No diagonal upcharge in edge cost | Binary's `AStar_compute_edge_cost` has no direction param; was the originally-named bug | Ghidra `0x00429830` signature |
| Task 5-7 | Uniform-cost step count, cliff penalty, Euclidean values | Confirm the fix lands without regressing cliff/cost-modifier behavior | New unit tests |

## Sources & References

- **Design doc:** [docs/plans/2026-05-07-astar-uniform-cost-design.md](docs/plans/2026-05-07-astar-uniform-cost-design.md)
- **Ghidra reports:**
  - `ra2-rust-game-docs/PATHFINDING_CELL_ENTRY_VERIFICATION_REPORT.md` (§1.1, §1.3, §1.5, §6.2)
  - `ra2-rust-game-docs/PATHFINDING_ASTAR_GHIDRA_REPORT.md` (referenced)
  - `ra2-rust-game-docs/PATHFINDERCLASS_GHIDRA_REPORT.md` (referenced)
- **gamemd.exe addresses (kept here, NOT in Rust comments per project rule):**
  - `AStar_main_loop @ 0x00429a90`
  - `AStar_create_node @ 0x0042a460` (Euclidean heuristic)
  - `AStar_compute_edge_cost @ 0x00429830` (no direction parameter)
  - Base cost table `0x0081870c` (8 floats)
  - Direction tiebreaker table `0x0081872c` (8 floats, 0.001..0.008)
  - Cliff multiplier constant `DAT_007e37bc` = 4.0f
- **Related code:** [src/sim/pathfinding/core.rs](src/sim/pathfinding/core.rs), [src/sim/pathfinding/core_tests.rs](src/sim/pathfinding/core_tests.rs)

---

## Tasks

### Task 1: Replace cost constants and cost-selector site

**Why:** First atomic step of the core swap. Constants change shape; downstream code must compile after each step but tests only need to be green at end of Task 4.

**Files:**
- Modify: [src/sim/pathfinding/core.rs:27-31](src/sim/pathfinding/core.rs#L27-L31) — constants
- Modify: [src/sim/pathfinding/core.rs:543-547](src/sim/pathfinding/core.rs#L543-L547) — cost selector
- Modify: [src/sim/pathfinding/core.rs:45-46](src/sim/pathfinding/core.rs#L45-L46) — comment referring to `CARDINAL_COST=10`

**Pattern:** Replaces existing private-constant block; same location, same shape.

**Step 1: Replace constants block.**

In [src/sim/pathfinding/core.rs](src/sim/pathfinding/core.rs) at lines 27-31, replace:

```rust
/// Movement cost for a cardinal step (N, E, S, W). Scaled by 10 for integer math.
const CARDINAL_COST: i32 = 10;

/// Movement cost for a diagonal step (NE, SE, SW, NW). Approximates sqrt(2) * 10.
const DIAGONAL_COST: i32 = 14;
```

With:

```rust
/// Uniform A* edge cost for all 8 compass directions. The original engine has
/// no diagonal upcharge — `AStar_compute_edge_cost` takes no direction
/// parameter. The 1000× scale is chosen so DIR_TIEBREAK [1..=8] sits at
/// exactly 0.001..0.008 of base, matching the binary's tiebreaker ratio.
const STEP_COST: i32 = 1000;
```

**Step 2: Update the cliff-multiplier comment.**

At lines 44-46:

```rust
/// Cost multiplier for cells with height transitions (ramps, slopes).
/// With CARDINAL_COST=10, a height step costs 40 instead of 10.
const CLIFF_COST_MULTIPLIER: i32 = 4;
```

Replace the `With ...` line:

```rust
/// Cost multiplier for cells with height transitions (ramps, slopes).
/// With STEP_COST=1000, a height step costs 4000 instead of 1000.
const CLIFF_COST_MULTIPLIER: i32 = 4;
```

**Step 3: Update the cost selector at lines 543-547.**

Replace:

```rust
            // Step cost
            let base_cost = if is_diagonal {
                DIAGONAL_COST
            } else {
                CARDINAL_COST
            };
```

With:

```rust
            // Step cost — uniform across all 8 compass directions.
            // The is_diagonal flag is still consumed by the corner-cutting
            // check above; only the cost is unified.
            let base_cost = STEP_COST;
```

**Step 4: Verify it compiles.**

Run: `cargo check -p ra2-rust-game --lib`

Expected: build fails — `octile_heuristic` still references `CARDINAL_COST` and `DIAGONAL_COST`. That's resolved in Task 2. Do NOT commit yet.

---

### Task 2: Replace octile heuristic with Euclidean

**Why:** Heuristic body and call sites must change together so the file compiles. After this task plus Task 1, the file compiles but heuristic tests still fail (resolved in Task 3).

**Files:**
- Modify: [src/sim/pathfinding/core.rs:1115-1126](src/sim/pathfinding/core.rs#L1115-L1126) — heuristic body
- Modify: [src/sim/pathfinding/core.rs:346](src/sim/pathfinding/core.rs#L346) — call site (start node)
- Modify: [src/sim/pathfinding/core.rs:586](src/sim/pathfinding/core.rs#L586) — call site (neighbor expansion)

**Pattern:** Replaces existing private function in-place; same signature, different body.

**Step 1: Replace `octile_heuristic` body and rename.**

At lines 1115-1126, replace:

```rust
/// Octile distance heuristic for 8-directional grid movement.
///
/// Consistent (never overestimates) with diagonal cost = 14, cardinal cost = 10.
/// Formula: max(dx, dy) * CARDINAL_COST + (min(dx, dy)) * (DIAGONAL_COST - CARDINAL_COST)
fn octile_heuristic(ax: u16, ay: u16, bx: u16, by: u16) -> i32 {
    let dx: i32 = (ax as i32 - bx as i32).abs();
    let dy: i32 = (ay as i32 - by as i32).abs();
    let min_d: i32 = dx.min(dy);
    let max_d: i32 = dx.max(dy);
    // max_d cardinal steps + min_d upgrades from cardinal to diagonal.
    max_d * CARDINAL_COST + min_d * (DIAGONAL_COST - CARDINAL_COST)
}
```

With:

```rust
/// Euclidean distance heuristic for 8-directional grid movement.
///
/// Matches the original engine's per-node heuristic computation —
/// sqrt(dx² + dy²) — applied at A* node creation time.
///
/// Intentionally **inadmissible** under uniform edge cost: when both dx and
/// dy are nonzero, Euclidean overestimates the true minimum path cost
/// (which is `max(dx, dy) * STEP_COST` since one diagonal step covers
/// both axes for the same price). The original engine accepts this
/// inadmissibility — A* trades optimality for shorter expansion, and
/// the resulting path geometry is part of the engine's character.
///
/// Implementation: scaled integer sqrt at full precision. `u64`
/// intermediate avoids overflow on 512×512 maps.
fn euclidean_heuristic(ax: u16, ay: u16, bx: u16, by: u16) -> i32 {
    let dx = (ax as i64 - bx as i64).unsigned_abs();
    let dy = (ay as i64 - by as i64).unsigned_abs();
    let sum_sq: u64 = dx * dx + dy * dy;
    // sqrt(sum_sq) * STEP_COST, computed as sqrt(sum_sq * STEP_COST²) for full precision.
    (sum_sq * 1_000_000).isqrt() as i32
}
```

**Step 2: Rename call site at line 346.**

```rust
        f_cost: octile_heuristic(start.0, start.1, goal.0, goal.1),
```

→

```rust
        f_cost: euclidean_heuristic(start.0, start.1, goal.0, goal.1),
```

**Step 3: Rename call site at line 586.**

```rust
                let h = octile_heuristic(nx, ny, goal.0, goal.1);
```

→

```rust
                let h = euclidean_heuristic(nx, ny, goal.0, goal.1);
```

**Step 4: Verify it compiles.**

Run: `cargo check -p ra2-rust-game --lib`

Expected: PASS. Tests will still fail; that's Task 3.

---

### Task 3: Update existing heuristic-test expectations

**Why:** The 3 existing tests in `core_tests.rs` reference the old function name and old expected values. After Task 2 they won't compile (function rename) and after Task 1 they won't pass (value change). Both reasons resolved here.

**Files:**
- Modify: [src/sim/pathfinding/core_tests.rs:40-58](src/sim/pathfinding/core_tests.rs#L40-L58)

**Pattern:** In-place test update. Function name + expected integer.

**Step 1: Replace `test_octile_heuristic_cardinal` (lines 40-45).**

```rust
#[test]
fn test_octile_heuristic_cardinal() {
    // Straight horizontal: 5 steps × 10 = 50.
    let h: i32 = octile_heuristic(0, 0, 5, 0);
    assert_eq!(h, 50);
}
```

→

```rust
#[test]
fn test_euclidean_heuristic_cardinal() {
    // Pure cardinal: sqrt(25) * 1000 = 5000.
    let h: i32 = euclidean_heuristic(0, 0, 5, 0);
    assert_eq!(h, 5000);
}
```

**Step 2: Replace `test_octile_heuristic_diagonal` (lines 47-52).**

```rust
#[test]
fn test_octile_heuristic_diagonal() {
    // Pure diagonal: 3 steps × 14 = 42.
    let h: i32 = octile_heuristic(0, 0, 3, 3);
    assert_eq!(h, 42);
}
```

→

```rust
#[test]
fn test_euclidean_heuristic_diagonal() {
    // Pure diagonal (3,3): sqrt(18) * 1000 ≈ 4242.64;
    // isqrt(18_000_000) = 4242 (4242² = 17_994_564, 4243² = 18_003_049).
    let h: i32 = euclidean_heuristic(0, 0, 3, 3);
    assert_eq!(h, 4242);
}
```

**Step 3: Replace `test_octile_heuristic_mixed` (lines 54-58).**

```rust
#[test]
fn test_octile_heuristic_mixed() {
    // dx=5, dy=3: 3 diagonal + 2 cardinal = 3*14 + 2*10 = 62.
    let h: i32 = octile_heuristic(0, 0, 5, 3);
    assert_eq!(h, 62);
}
```

→

```rust
#[test]
fn test_euclidean_heuristic_mixed() {
    // dx=5, dy=3: sqrt(34) * 1000 ≈ 5830.95;
    // isqrt(34_000_000) = 5830 (5830² = 33_988_900, 5831² = 34_000_561).
    let h: i32 = euclidean_heuristic(0, 0, 5, 3);
    assert_eq!(h, 5830);
}
```

---

### Task 4: Run pathfinding tests, address any path-geometry regressions

**Why:** Tasks 1-3 form one atomic change but several existing tests assert specific path geometry. With uniform cost, paths returned for the same input may differ in intermediate cells (start/goal/length usually preserved). This task surfaces and resolves any such regressions before commit.

**Files:** [src/sim/pathfinding/core_tests.rs](src/sim/pathfinding/core_tests.rs) — fix any failing test assertion to reflect new (correct) behavior.

**Step 1: Run pathfinding tests.**

Run: `cargo test -p ra2-rust-game --lib pathfinding`

**Step 2: Categorize each failure.**

For each failing test, decide between:
- **Update the assertion** (path geometry shifted but new geometry is correct under uniform cost — e.g. an obstacle-routing test that previously expected a specific intermediate cell).
- **Investigate** (failure indicates an unintended bug, not an expected geometry shift). Stop and re-read the test before changing it.

Tests *not* expected to fail (verified by reading them):
- `test_path_grid_*` (walkability) — independent of cost.
- `test_find_path_trivial_same_cell` — trivial.
- `test_find_path_straight_line` — asserts only first/last and length.
- `test_find_path_diagonal` — asserts only first/last and length=4 (Chebyshev still gives 4).
- `test_find_path_around_obstacle` — asserts cells avoid the wall column, not specific path.
- `test_find_path_no_path_exists` — independent of cost.
- `test_find_path_blocked_*` — asserts only existence + first/last.
- `test_find_path_no_diagonal_corner_cutting` — asserts no path; corner-cutting flag unchanged.
- `test_find_path_long_gets_full_result` — asserts length = 41 on open grid (Chebyshev still gives 41).
- `test_block_building_footprint` etc. — walkability only.
- `test_layered_path_*` — bridge geometry; cliff cost still applies; assertions are layer-based.
- `test_entity_blocks_*` — assert routing avoids cell or asserts existence; no specific geometry.
- `code2_*` — assert chain-walk multipliers; multipliers unchanged.
- Water/naval tests — assert layer/row constraints, not specific cells.

If a previously-passing test fails on a *cell-specific* assertion, fix the assertion to match the new path; the new path is the correct one per the design.

**Step 3: Re-run until green.**

Run: `cargo test -p ra2-rust-game --lib pathfinding`

Expected: PASS.

**Step 4: Commit.**

Run:
```
git add src/sim/pathfinding/core.rs src/sim/pathfinding/core_tests.rs
git commit -m "$(cat <<'EOF'
pathfinding: uniform step cost + Euclidean heuristic

Replace octile A* (cardinal=10, diagonal=14, octile heuristic) with
gamemd.exe's uniform-cost / Euclidean-heuristic model. STEP_COST=1000
keeps DIR_TIEBREAK [1..8] at the binary's exact 0.001..0.008-of-base ratio.
Heuristic is sqrt(dx² + dy²) via integer u64::isqrt.

Verified against AStar_compute_edge_cost (no direction parameter) and
AStar_create_node (Sqrt_Approx of squared distance).
EOF
)"
```

---

### Task 5: Add `test_euclidean_heuristic_zero`

**Why:** Boundary case. Heuristic at goal cell must be 0. Pins behavior even though it's also implicit in trivial-path tests.

**Files:**
- Modify: [src/sim/pathfinding/core_tests.rs](src/sim/pathfinding/core_tests.rs) — add test next to the other heuristic tests.

**Step 1: Add test after `test_euclidean_heuristic_mixed`.**

```rust
#[test]
fn test_euclidean_heuristic_zero() {
    // Heuristic at goal cell must be 0 — guarantees A* can recognize the
    // goal as the lowest-f node when popped.
    let h: i32 = euclidean_heuristic(7, 7, 7, 7);
    assert_eq!(h, 0);
}
```

**Step 2: Verify.**

Run: `cargo test -p ra2-rust-game --lib pathfinding::core_tests::test_euclidean_heuristic_zero`

Expected: PASS.

---

### Task 6: Add `test_path_step_count_chebyshev_open_grid`

**Why:** Locks the property "uniform cost picks fewest-step paths on open grids." The Euclidean heuristic still steers toward diagonals, so the returned step count should equal `max(|dx|, |dy|)`.

**Files:**
- Modify: [src/sim/pathfinding/core_tests.rs](src/sim/pathfinding/core_tests.rs) — add after the existing path tests, before the bridge tests.

**Step 1: Add the test.**

```rust
#[test]
fn test_path_step_count_chebyshev_open_grid() {
    // With uniform edge cost and a Euclidean heuristic, the optimal step
    // count from (sx,sy) to (gx,gy) on an open grid is max(|dx|, |dy|).
    // Path length includes the start cell, so it equals chebyshev + 1.
    let grid: PathGrid = PathGrid::new(20, 20);
    let cases: &[((u16, u16), (u16, u16), usize)] = &[
        ((0, 0), (5, 0), 6),   // pure cardinal: 5 E steps -> 6 cells
        ((0, 0), (0, 7), 8),   // pure cardinal: 7 S steps -> 8 cells
        ((0, 0), (4, 4), 5),   // pure diagonal: 4 SE steps -> 5 cells
        ((0, 0), (5, 3), 6),   // mixed: max(5,3) = 5 -> 6 cells
        ((0, 0), (7, 2), 8),   // mixed: max(7,2) = 7 -> 8 cells
        ((10, 10), (3, 15), 8), // both axes nonzero, dx=7 dy=5 -> 8 cells
    ];
    for &(start, goal, expected_len) in cases {
        let path = find_path(&grid, start, goal)
            .unwrap_or_else(|| panic!("no path for {:?}->{:?}", start, goal));
        assert_eq!(
            path.len(),
            expected_len,
            "path {:?}->{:?} expected {} cells, got {}: {:?}",
            start,
            goal,
            expected_len,
            path.len(),
            path
        );
        assert_eq!(path.first().copied(), Some(start));
        assert_eq!(path.last().copied(), Some(goal));
    }
}
```

**Step 2: Verify.**

Run: `cargo test -p ra2-rust-game --lib pathfinding::core_tests::test_path_step_count_chebyshev_open_grid`

Expected: PASS.

---

### Task 7: Add `test_cliff_cost_detours_under_uniform_base`

**Why:** Cliff multiplier (4×) is preserved across this change. Confirm a height-step path is still penalized enough that A* prefers a flat alternative when one exists.

**Files:**
- Modify: [src/sim/pathfinding/core_tests.rs](src/sim/pathfinding/core_tests.rs) — add near other path-routing tests.

**Step 1: Add the test.**

```rust
#[test]
fn test_cliff_cost_detours_under_uniform_base() {
    // 3x3 grid. Direct row 0 has a height-step at (1,0): ground=4 between
    // ground=0 cells. Direct path is 2 steps but pays cliff×4 twice
    // (entering and leaving) ≈ 10000 g_cost. Alt path goes via row 1
    // (all flat): 4 steps ≈ 4000 g_cost. Alt should win.
    let cells = vec![
        // Row 0 (y=0): 0, [cliff height=4], 0
        PathCell { ground_walkable: true, bridge_walkable: false, transition: false, ground_level: 0, bridge_deck_level: 0 },
        PathCell { ground_walkable: true, bridge_walkable: false, transition: false, ground_level: 4, bridge_deck_level: 0 },
        PathCell { ground_walkable: true, bridge_walkable: false, transition: false, ground_level: 0, bridge_deck_level: 0 },
        // Row 1 (y=1): all flat 0
        PathCell { ground_walkable: true, bridge_walkable: false, transition: false, ground_level: 0, bridge_deck_level: 0 },
        PathCell { ground_walkable: true, bridge_walkable: false, transition: false, ground_level: 0, bridge_deck_level: 0 },
        PathCell { ground_walkable: true, bridge_walkable: false, transition: false, ground_level: 0, bridge_deck_level: 0 },
        // Row 2 (y=2): all flat 0 (filler so 3x3)
        PathCell { ground_walkable: true, bridge_walkable: false, transition: false, ground_level: 0, bridge_deck_level: 0 },
        PathCell { ground_walkable: true, bridge_walkable: false, transition: false, ground_level: 0, bridge_deck_level: 0 },
        PathCell { ground_walkable: true, bridge_walkable: false, transition: false, ground_level: 0, bridge_deck_level: 0 },
    ];
    let grid = PathGrid::from_cells(cells, 3, 3);
    let path = find_path(&grid, (0, 0), (2, 0))
        .expect("path should exist over flat alt route");
    // Direct path through cliff would visit (1,0). Alt route avoids it.
    assert!(
        !path.contains(&(1, 0)),
        "path should detour around cliff cell (1,0): {:?}",
        path
    );
    assert_eq!(path.first().copied(), Some((0, 0)));
    assert_eq!(path.last().copied(), Some((2, 0)));
}
```

**Step 2: Verify.**

Run: `cargo test -p ra2-rust-game --lib pathfinding::core_tests::test_cliff_cost_detours_under_uniform_base`

Expected: PASS.

---

### Task 8: Run full pathfinding suite + commit

**Why:** Confirm the new tests don't interact badly with the rest of the suite, then land them as a single parity-test commit.

**Step 1: Run full pathfinding suite.**

Run: `cargo test -p ra2-rust-game --lib pathfinding`

Expected: ALL PASS.

**Step 2: Run the full library suite once.**

Run: `cargo test -p ra2-rust-game --lib`

Expected: ALL PASS. (Pathfinding is upstream of movement, so any silent regression elsewhere shows up here.)

**Step 3: Commit.**

Run:
```
git add src/sim/pathfinding/core_tests.rs
git commit -m "$(cat <<'EOF'
pathfinding: parity tests for uniform-cost A*

Add three regression tests that pin the new uniform-cost / Euclidean-
heuristic behavior:
- euclidean_heuristic returns 0 at the goal cell
- open-grid path step count equals Chebyshev distance + 1
- cliff multiplier still detours when a flat alt route exists
EOF
)"
```

---

### Task 9: gamemd.exe in-game parity verification

**Why:** Unit tests pin math + step counts but the parity bar is "indistinguishable in a single skirmish." This is a manual verification step.

**Step 1: Pick a comparison scenario.**

Boot a YR skirmish in gamemd.exe and the Rust engine on the same map (e.g. a stock map with mixed terrain — Country Swing or similar). Issue the same move order to a single tank from one side of the map to the other, near terrain features that historically drove path choice (cliffs, narrow passes, ore fields).

**Step 2: Compare path shapes.**

- Same start, same goal, same map → same path geometry.
- Multi-unit move orders should split into the same per-unit destinations.
- Watch for: direction preference at first step, cell-by-cell route through narrow passes, behavior when goal is across a cliff face.

**Step 3: Document any drift.**

If the Rust path diverges from gamemd's path in a way the player would notice, capture:
- Map name + start/goal cell coordinates
- Screenshot of both paths
- Hypothesis (likely culprit: a still-unimplemented soft-block code, an unhandled cliff-flag, a tiebreaker direction we've gotten wrong)

Open a follow-up `/disparity-scan pathfinding` or `/re-investigate` rather than fixing on the spot — the fix is out of this plan's scope.

**Step 4: If no drift observed, mark complete.**

This is the parity bar. Single-skirmish observable similarity is the success condition.

---

## Post-Plan Self-Review

1. **Spec coverage** — every design-doc requirement has a task: STEP_COST (T1), no direction multiplier (T1), Euclidean heuristic (T2), call-site renames (T2), test updates (T3), parity tests (T5/T6/T7), gamemd verification (T9). ✓
2. **Placeholder scan** — no TBD/TODO/vague steps. ✓
3. **Architecture check** — single-file change inside sim/, no cross-module dependencies. ✓
4. **Interface ordering** — no new public interfaces; only private renames within the same file (declaration and call sites updated together in T2). ✓
5. **Risk coverage** — T4 explicitly addresses the highest risk (existing tests with cell-specific assertions). T9 covers in-game parity. ✓
6. **Self-containment** — each task names exact files, line ranges, and full code. ✓
7. **Sim/ compliance** — sim checklist in plan header. No render/ui/audio/net deps. ✓
8. **Grounding coverage** — Ghidra (live verified this session), docs (cited), repo pattern (file/line refs), INI (n/a — confirmed no INI keys involved). ✓
9. **Confidence tagging** — all decisions marked high. ✓
10. **Deferred questions** — Q listed about path-geometry regressions in Task 4. ✓
11. **Parity-critical items** — table populated with 4 entries linking tasks to verification. ✓
