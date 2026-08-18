# A* Uniform Edge Cost Design

## Goal

Replace our octile A* (cardinal=10, diagonal=14, octile heuristic) with gamemd.exe's
uniform-cost / Euclidean-heuristic model so the pathfinder returns the same paths
the original engine does.

## Architecture Context

A* lives in a single file: [src/sim/pathfinding/core.rs](src/sim/pathfinding/core.rs).
Constants, heuristic, and the main loop are co-located. Public callers
(`find_path`, `find_path_with_costs`, `find_path_with_costs_corridor`,
`find_layered_path`) only consume the returned cell sequence — none look at
absolute cost values, so the change is internal to this file.

Determinism: `AStarNode::cmp` breaks ties on `f_cost → -g_cost → y → x`.
State-hash inputs are entity positions, not pathfinder internals. Replays
encode commands + RNG, not paths — but path divergence will produce
divergent unit positions tick-by-tick, so old replays will desync. Acceptable.

## Impact Analysis

- **Touched code:** 4 constants + 1 heuristic body + 1 cost-selector site, all in
  [src/sim/pathfinding/core.rs](src/sim/pathfinding/core.rs).
- **Tests:** 3 heuristic tests in
  [src/sim/pathfinding/core_tests.rs:41-58](src/sim/pathfinding/core_tests.rs#L41-L58)
  need updated expectations; new parity tests added.
- **Blast radius:** sim only. No render/UI/audio coupling. No save/load format change.
- **Replays:** state hashes diverge from pre-change replays. Expected.
- **Net / lockstep:** unchanged contract — all clients run the same code version.
  Integer math throughout.
- **Adjacent parity gaps observed but explicitly out of scope** (see Alternatives):
  - Code 4 (cost 60.0) appears unimplemented; doc has internal disagreement on
    its meaning. Needs separate `/re-investigate`.
  - Bridge flanking diagonal multipliers (1.0/2.0/10.0) per doc §1.4 unimplemented.
  - `PathfinderClass+0x04` cost multiplier (binary scales every edge cost by it)
    unimplemented.

## Chosen Approach

Switch to a single uniform `STEP_COST = 1000` and replace the octile heuristic with
integer Euclidean. The 1000 scalar is chosen so `DIR_TIEBREAK = [1..=8]` sits at
exactly 0.001–0.008 of base — matching the binary's tiebreaker ratio at
`DAT_0081872c` (0.001f..0.008f against base 1.0f).

Why this scalar and not 10:

- Base=10 with our existing tiebreak [1..=8] gives a 100× exaggerated tiebreaker
  ratio relative to the binary. Under uniform edge cost, more paths share the
  same f_cost — the tiebreaker becomes the path-selector. A 100×-strong tiebreaker
  picks paths the binary wouldn't. Player-visible.
- Base=1000 keeps the tiebreaker ratio exact at no implementation cost and same
  diff size.
- Cliff-multiplier (4×) and code multipliers (4/8/20/1000) work unchanged — they
  multiply step_cost.

Heuristic body (full-precision integer Euclidean):

```rust
fn euclidean_heuristic(ax: u16, ay: u16, bx: u16, by: u16) -> i32 {
    let dx = (ax as i64 - bx as i64).unsigned_abs();
    let dy = (ay as i64 - by as i64).unsigned_abs();
    let sum_sq: u64 = dx * dx + dy * dy;
    // h_binary = sqrt(dx² + dy²) at base 1.0; we operate at 1000×.
    // sqrt(sum_sq * 1_000_000) keeps full integer precision.
    (sum_sq * 1_000_000).isqrt() as i32
}
```

This is intentionally **inadmissible** under uniform cost: Euclidean overestimates
the true minimum path cost (Chebyshev) when both dx and dy are nonzero. The
binary accepts the inadmissibility — A* trades optimality for shorter expansion,
which is part of gamemd's path-shape character. Verified from binary at
[`AStar_create_node` 0x0042a460](file:///).

## Tiny-Detail Ledger

| Item | Value | Source |
|------|-------|--------|
| Base step cost (uniform across all 8 dirs) | 1000 (binary 1.0 × 1000) | [GHIDRA `0x0081870c` mem read; doc §1.1] |
| Per-direction multiplier | NONE | [GHIDRA `AStar_compute_edge_cost` 0x429830 — function takes no direction parameter] |
| Heuristic = √(dx² + dy²) (Euclidean) | inadmissible under uniform cost — preserved | [GHIDRA `AStar_create_node` 0x0042a460 — `Sqrt_Approx(dx² + dy²)`] |
| f_cost = g_cost + h | f computed at node-create time | [GHIDRA `AStar_create_node` 0x0042a460] |
| g_cost = parent.g + (edge_cost × cost_mult + tiebreaker) | tiebreaker accumulates into g | [GHIDRA `AStar_main_loop` 0x429a90, `AStar_create_node` 0x0042a460] |
| Cliff ramp multiplier | 4.0× — applied to step_cost | [doc §1.3, `DAT_007e37bc`] |
| Direction tiebreaker (N..NW) | 0.001..0.008 floats added per step | [doc §1.5, table at `0x0081872c`] |
| Code 2 cost (moving friendly, after prediction) | 4.0 (jam) / 1000.0 (destroyer) | [doc §1.2] |
| Code 5 cost (enemy unit) | 20.0 | [doc §1.1] |
| Code 6 cost (per binary table — labeled "Cliff" in §1.1, "Stationary friendly" in §6.3) | 8.0 | [doc §1.1, §6.3 — internal disagreement; matches our impl regardless] |
| 8-direction order N, NE, E, SE, S, SW, W, NW | matches binary's `g_CellNeighborOffsets_8Dir` | (preserved) |
| Diagonal corner-cutting blocked when both flank cardinals are blocked | preserved as-is | (preserved) |
| Code 4 cost (60.0) | UNIMPLEMENTED — known parity gap, separate item | [doc §1.1] |
| Bridge flanking diagonal multipliers (1.0/2.0/10.0) | UNIMPLEMENTED — known parity gap, separate item | [doc §1.4] |
| `PathfinderClass+0x04` global cost multiplier | UNIMPLEMENTED — known parity gap, separate item | [doc §1.5] |

## Design

### Components

Single file change: [src/sim/pathfinding/core.rs](src/sim/pathfinding/core.rs).

**Constant changes:**

```rust
// REMOVE
const CARDINAL_COST: i32 = 10;
const DIAGONAL_COST: i32 = 14;

// ADD
/// Uniform A* edge cost for all 8 compass directions.
/// gamemd.exe uses 1.0 in `0x0081870c[0]`; we use 1000 so DIR_TIEBREAK [1..8]
/// sits at the binary's exact 0.001..0.008-of-base ratio.
const STEP_COST: i32 = 1000;
```

`CLIFF_COST_MULTIPLIER = 4` and `CODE2_*` / `CODE5_*` / `CODE6_*` multipliers
keep their numeric values — they multiply `step_cost`, so the scale change passes
through proportionally.

The `DIR_TIEBREAK` array values [1..=8] are unchanged. Their ratio relative to
base now matches the binary's table at `0x0081872c` (1/1000..8/1000 ≈ 0.001..0.008).

**Cost-selector change** ([core.rs:543-547](src/sim/pathfinding/core.rs#L543-L547)):

```rust
// BEFORE
let base_cost = if is_diagonal { DIAGONAL_COST } else { CARDINAL_COST };

// AFTER
let base_cost = STEP_COST;
```

The `is_diagonal` flag is still used by the corner-cutting check above this line
[core.rs:508-540](src/sim/pathfinding/core.rs#L508-L540) — preserved.

**Heuristic change** ([core.rs:1115-1126](src/sim/pathfinding/core.rs#L1115-L1126)):

```rust
/// Euclidean distance heuristic — matches gamemd.exe's `Sqrt_Approx(dx² + dy²)`
/// inside `AStar_create_node` (0x0042a460). Inadmissible under uniform edge
/// cost; preserved deliberately to match binary path-selection behavior.
fn euclidean_heuristic(ax: u16, ay: u16, bx: u16, by: u16) -> i32 {
    let dx = (ax as i64 - bx as i64).unsigned_abs();
    let dy = (ay as i64 - by as i64).unsigned_abs();
    let sum_sq: u64 = dx * dx + dy * dy;
    (sum_sq * 1_000_000).isqrt() as i32
}
```

Both call sites ([core.rs:346, 586](src/sim/pathfinding/core.rs#L346)) renamed
`octile_heuristic` → `euclidean_heuristic`.

### Data Flow

Unchanged. A* fills g_cost arrays, expands neighbors, pushes onto BinaryHeap, pops
lowest f_cost first, terminates on goal. Only the absolute values of the costs change.

### Interfaces / Contracts

Public functions (`find_path`, `find_path_with_costs`, `find_path_with_costs_corridor`,
`find_layered_path`, `astar_search`) keep their signatures. Return types unchanged.

### Error Handling

Unchanged. Search exhaustion (`MAX_SEARCH_NODES`) and "no path" return `None`.
Goal-fallback near-miss heuristic at [core.rs:455](src/sim/pathfinding/core.rs#L455)
unchanged (uses height comparison, not cost).

### Integer Overflow Analysis

Worst case for 512×512 maps:
- `dx = dy = 511` → `sum_sq = 522,242`
- `sum_sq × 1_000_000 = 5.22 × 10¹¹` — fits u64 trivially.
- `isqrt(5.22 × 10¹¹) ≈ 722,664` — fits i32 easily.

g_cost max: a 65,527-node search at worst ~650,000 cost per step (cliff × code-2
worst case ≈ 1000 × 4 × 4 = 16,000 plus tiebreaker) → ~1.05 × 10⁹, fits i32 (max
~2.15 × 10⁹). Safe with the existing `MAX_SEARCH_NODES` cap.

f_cost max = g_cost + heuristic max ≈ 1.05 × 10⁹ + 7.23 × 10⁵ → still i32-safe.

### Determinism

- All math integer (`u64` for the heuristic intermediate, `i32` everywhere else).
- `u32::isqrt` / `u64::isqrt` are deterministic across platforms (pure integer
  algorithm).
- `AStarNode::cmp` already deterministic; preserved.

### Testing Strategy

**Update existing heuristic tests** in
[src/sim/pathfinding/core_tests.rs](src/sim/pathfinding/core_tests.rs):

- `test_octile_heuristic_cardinal` → `test_euclidean_heuristic_cardinal`:
  expects `5 × STEP_COST = 5000` (was 50).
- `test_octile_heuristic_diagonal` → `test_euclidean_heuristic_diagonal`:
  expects `isqrt(18 × 1_000_000) = 4242` (was 42).
- `test_octile_heuristic_mixed` → `test_euclidean_heuristic_mixed`:
  for (5,3): `isqrt(34 × 1_000_000) = 5830` (was 62).

**Add new parity tests:**

1. `test_uniform_edge_cost_diag_equals_cardinal` — A* graph where the only path
   options are 1 diagonal vs 1 cardinal. Both should have identical step cost
   (modulo direction tiebreaker). Verify by constructing nodes and checking
   `current.g_cost + step_cost` is identical for both.

2. `test_path_prefers_diagonal_via_heuristic` — open 5×5 grid, path (0,0) → (3,3).
   Expect 4-cell path (start + 3 diagonals), confirming the Euclidean heuristic
   still steers toward shorter diagonals despite uniform edge cost.

3. `test_path_step_count_matches_chebyshev_on_open_grid` — for several (start,
   goal) pairs on an open grid, returned path length equals
   `max(|dx|, |dy|) + 1`. Confirms uniform cost picks fewest-step paths.

4. `test_direction_tiebreak_preference_under_equal_fcost` — start (0,0), goal
   (2,2). Multiple equal-cost diagonal paths exist (NE→NE, mixed). Verify the
   path returned matches the direction-preference order
   `N < NE < E < SE < S < SW < W < NW` from `DIR_TIEBREAK`. Key parity test.

5. `test_cliff_cost_still_applied_with_uniform_base` — height-step path; verify
   cliff multiplier still penalizes effectively (path detours around steep
   terrain when alternative exists).

6. `test_existing_obstacle_routing_still_works` — keep regression coverage for
   existing tests (`test_find_path_around_obstacle`, `test_find_path_no_path_exists`,
   etc.) — they should pass unchanged because path *existence* is unaffected.

7. `test_heuristic_inadmissibility_does_not_break_search` — pathological case
   where Euclidean overestimates significantly: long L-shape detour. A* should
   still find *a* path; we don't assert it's optimal (it won't be) — we assert
   the search completes and returns a connected path.

## Architectural Decisions

- **Single file, single PR scope.** Pattern matches recent pathfinding fixes —
  contained changes with clear blast radius.
- **No fixed-point migration.** sim/ wants fixed-point eventually (CLAUDE.md), but
  i32 is what `g_cost` already is and changing the storage type is a separate
  refactor with its own tradeoffs (see Alternatives).
- **Heuristic kept inadmissible.** Deliberately matching binary behavior over
  textbook A* correctness. Documented in the heuristic comment.
- **`u32::isqrt` / `u64::isqrt` dependency** — stabilized in Rust 1.84
  (Jan 2025). Verify Cargo.toml MSRV (or rust-toolchain.toml) covers this
  before implementation; if not, either pin the toolchain or implement a
  small integer-sqrt helper.

## Alternatives Considered

**Approach 1 (base=10, defer tiebreaker fix).** Smaller scalar — less disruption.
Rejected: leaves a 100×-too-strong tiebreaker as a known parity drift, when fixing
it costs nothing extra. Not faithful.

**Approach 3 (full fixed-point refactor with `fixed::I16F16`).** Most faithful to
binary's float math; aligns with sim/ standard. Rejected: scope explosion —
touches `AStarNode` ordering, heap, and every g_cost / f_cost arithmetic site.
Worth doing eventually as a separate brainstorm.

**Bundling code-4 (cost 60) and bridge flanking multipliers.** Both are real
parity gaps near this code. Rejected here: code-4 has internal doc disagreement
(§1.1 vs §6.3) on whether it's wall/gate or stationary-friendly — needs
`/re-investigate`. Bridge flanking is a distinct system (only fires in
bridge-aware mode). Bundling either would mean shipping a guess. Each gets its
own scoped item.
