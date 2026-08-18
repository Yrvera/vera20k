# Path Smoothing — Straight-Line Trace

**Scenario:** Grizzly Tank (50,50) → (50,60), pure south, 10 cells, flat open grass, no obstacles.
**Mechanic:** Path smoothing — do the returned waypoints form a straight line (no zig-zag)?
**Date:** 2026-05-20

---

## 1. gamemd.exe Pipeline (ground-truth)

### 1.1 A* Expansion — Direction Encoding

Dir encoding at neighbor offset table `0x7e3774` (N=0 standard):
```
Dir 0 (N):  offset -512  (dy=-1, dx=0)
Dir 1 (NE): offset -511  (dy=-1, dx=+1)
Dir 2 (E):  offset +1    (dy=0,  dx=+1)
Dir 3 (SE): offset +513  (dy=+1, dx=+1)
Dir 4 (S):  offset +512  (dy=+1, dx=0)
Dir 5 (SW): offset +511  (dy=+1, dx=-1)
Dir 6 (W):  offset -1    (dy=0,  dx=-1)
Dir 7 (NW): offset -513  (dy=-1, dx=-1)
```
Source: `PATHFINDING_ASTAR_GHIDRA_REPORT.md §4.3`.

### 1.2 A* Raw Expansion — Flat Open Terrain

**Input:** start=(50,50), goal=(50,60), all cells cost=1.0 (Can_Enter_Cell→0), no diagonal
corner obstruction, no slopes.

**Heuristic:** Euclidean distance `sqrt(dx²+dy²)` from current node to goal.
Source: `PATHFINDING_ASTAR_GHIDRA_REPORT.md §4.6`.

**Direction tiebreaker** (from `DirectionEpsilon` table at 0x0081872c):
```
N(0)=0.001, NE(1)=0.005, E(2)=0.002, SE(3)=0.006
S(4)=0.003, SW(5)=0.007, W(6)=0.004, NW(7)=0.008
```

**Expansion analysis for pure-south path:**

From (50,50) expanding all 8 neighbors, the f-costs are:
- S  (50,51): g=1.003, h=sqrt(0+81)=9.0  → f=10.003
- SE (51,51): g=1.006+ε, h=sqrt(1+81)≈9.055 → f≈10.061
- SW (49,51): g=1.007+ε, h≈9.055 → f≈10.062
- N  (50,49): g=1.001, h=sqrt(0+121)=11.0 → f≈12.001
- E  (51,50): g=1.002, h=sqrt(1+100)≈10.05 → f≈11.052
- W  (49,50): g=1.004, h≈10.05 → f≈11.054

The S neighbor always dominates because both g-cost (smallest tiebreaker=0.003)
and h (shortest Euclidean distance) are minimized. The A* will expand the S
neighbor first at every step along the path.

**Raw direction array produced by `AStar_reconstruct_path` (0x42aa90):**
```
[4, 4, 4, 4, 4, 4, 4, 4, 4, 4, -1]   (10 × S, then sentinel)
```
This is the raw output before any smoothing passes.

Source: Doc synthesis from `PATHFINDING_ASTAR_GHIDRA_REPORT.md §4.11`, direction
table at `0x7e3774`, tiebreaker analysis above.
CONFIDENCE (raw expansion): HIGH for direction encoding; expansion order is inferred
from heuristic + cost math, not directly decompiled for this exact case. YR-active: YES.

### 1.3 Pass 1 — Path_smooth_corners (0x42b210)

**Input:** [4, 4, 4, 4, 4, 4, 4, 4, 4, 4, -1]

Pass 1 looks for 90-degree zigzag pairs where the *first* direction is diagonal.
All 10 steps are direction 4 (S, cardinal). There is no direction change at all.
The zigzag anchor rule requires the previous direction to be diagonal (odd).
Cardinal directions clear the anchor to -1 (see `PATH_SMOOTHING_AND_SPEED_RAMPING_GHIDRA_REPORT.md §8.2`).

**Output of Pass 1:** [4, 4, 4, 4, 4, 4, 4, 4, 4, 4, -1] — **UNCHANGED.**

### 1.4 Pass 2 — Path_optimize_straight_segments (0x42b7f0)

**Input:** [4, 4, 4, 4, 4, 4, 4, 4, 4, 4, -1] — all steps S(4).

Pass 2 tracks cumulative displacement from segment start. For a run of uniform S steps:
- After step 1: cum_offset = (0,+1), chebyshev=1 (increasing — no regression)
- After step 2: cum_offset = (0,+2), chebyshev=2 (still increasing)
- ...continuing through all 10 steps...

Drift regression fires when `chebyshev < previous_best` — a "path curves back" signal.
A constant-south run only increases chebyshev monotonically. Regression never fires.
The final end-of-window sweep calls `FUN_0042be20` with the full (0,+10) displacement.
The reroute decomposes: diag_steps=min(0,10)=0, cardinal_steps=10, cardinal_dir=S(4).
The replacement is identical to the original: [4,4,4,4,4,4,4,4,4,4]. No change.

**Output of Pass 2:** [4, 4, 4, 4, 4, 4, 4, 4, 4, 4, -1] — **UNCHANGED.**

Source: `PATH_SMOOTHING_AND_SPEED_RAMPING_GHIDRA_REPORT.md §3, §8.5, §8.6`.

### 1.5 Final Waypoint List (gamemd.exe)

The smoothed direction array is copied into `FootClass::path_queue` (24 entries max)
at `this+0x5E0`. With 10 S-steps, the queue is:
```
path_queue: [4, 4, 4, 4, 4, 4, 4, 4, 4, 4, -1, ...]
```

In **cell coordinates**, this is:
```
(50,50) → (50,51) → (50,52) → ... → (50,60)   [11 waypoints, 10 steps]
```
That is a **straight vertical sequence with no zig-zag** — exactly 1 waypoint sequence
of pure S moves.

### 1.6 Locomotor Consumption

`DriveLocomotionClass::Process` (0x4B0500) reads from path_queue one direction at a
time. Each direction selects a drive track from the drive track table. Direction S
consistently maps to a pure-south drive track. The unit drives cell-to-cell south
along the straight line, no angular deviation.

Source: `PATH_SMOOTHING_AND_SPEED_RAMPING_GHIDRA_REPORT.md §6`, `DRIVE_TRACK_SYSTEM.md`.

---

## 2. Our Rust Implementation — Same Scenario

### 2.1 A* in `core.rs`

**Direction encoding** (`NEIGHBORS` array, line 147-156):
```rust
(0, -1, false), // N=0
(1, -1, true),  // NE=1
(1, 0, false),  // E=2
(1, 1, true),   // SE=3
(0, 1, false),  // S=4
(-1, 1, true),  // SW=5
(-1, 0, false), // W=6
(-1, -1, true), // NW=7
```
**MATCHES** gamemd.exe direction encoding. PASS.

**Heuristic** (`euclidean_heuristic` called at line 578): Euclidean distance.
**MATCHES** gamemd.exe §4.6. PASS.

**Raw path from A*:** For flat open terrain (50,50)→(50,60), the A* expands
S-neighbor first at each step (same f-cost analysis as §1.2). Raw result:
```
[(50,50),(50,51),(50,52),...,(50,60)]  — 11 cells, 10 S-steps
```
Equivalent to gamemd's [4,4,4,4,4,4,4,4,4,4] direction array.

### 2.2 Pass 1 — `smooth_path` (path_smooth.rs:95)

**Pass 1 inputs the 11-cell coordinate array.** For each consecutive triple, it
computes `direction_between` and checks `dir_diff == 2` with diagonal anchor.

For the straight south path:
- Every step is direction 4 (S, cardinal)
- `dir_diff(4, 4) == 0`, not 2 — no zigzag detected
- Even if diff were 2, the anchor check `is_diagonal_dir(d0)` requires d0 to be odd.
  S(4) is even — anchor check fails.

**Output of Pass 1:** [(50,50)...(50,60)] — **UNCHANGED.** MATCHES gamemd.

### 2.3 Pass 2 — `optimize_path` (path_smooth.rs:267)

**Pass 2** calls `find_drift_segment` looking for cross-product drift.

For a straight south path, step deltas are all (0,+1):
- cum_dx=0, cum_dy grows. ideal_dx=0, ideal_dy grows.
- cross = |cum_dx * ideal_dy - cum_dy * ideal_dx| = |0*k - k*0| = 0.
- drift_sq = 0 ≤ dist_sq * DRIFT_THRESHOLD. No drift detected.

`find_drift_segment` returns None. `optimize_path` exits without modification.

**Output of Pass 2:** [(50,50)...(50,60)] — **UNCHANGED.** MATCHES gamemd.

### 2.4 Final Path Delivered to Locomotor

After truncation at MAX_PATH_SEGMENT_STEPS=24 (well under 11 cells), the path is:
```
[(50,50),(50,51),(50,52),(50,53),(50,54),(50,55),(50,56),(50,57),(50,58),(50,59),(50,60)]
```
With layers all `MovementLayer::Ground`. Straight line, 11 waypoints, no zig-zag.

---

## 3. Stage-by-Stage Verdict Table

| Stage | gamemd.exe | Our Rust | Verdict |
|-------|-----------|----------|---------|
| Direction encoding N=0..NW=7 | 0x7e3774 offset table | NEIGHBORS array in core.rs:147 | PASS |
| A* heuristic (Euclidean) | AStar_create_node 0x42a460 | euclidean_heuristic in core.rs | PASS |
| Raw path for pure-south move | All S-steps, no diagonal | All S-steps, same | PASS |
| Pass 1: diagonal anchor rule | Only odd dirs anchor (§8.2) | `is_diagonal_dir(d0)` gate at line 119 | PASS |
| Pass 1: cardinal zigzag preserved | N→E stays 2 steps | test `smooth_cardinal_zigzag_n_then_e_unchanged` confirms | PASS |
| Pass 1: straight path unchanged | All S → no zigzag | All S → no zigzag | PASS |
| Pass 2: chebyshev regression trigger | Chebyshev decreasing | Cross-product drift (different method) | DISPARITY (§4) |
| Pass 2: straight path unchanged | All S → no optimization | Cross=0 → no drift → unchanged | PASS (correct output, different trigger) |
| Pass 2: steep-slope threshold | 0.01 (verified §8.4) | Not implemented (flat-terrain closure) | NOT-IMPLEMENTED (no impact on flat terrain) |
| Pass 2: two-ordering retry | diagonal-first then cardinal-first | Single ordering (Bresenham interleave) | DISPARITY (§4) |
| Max steps limit (20) | 0x13 < iVar13 at 0x42b7f0 | MAX_OPTIMIZE_STEPS=20 in path_smooth.rs:254 | PASS |
| Path queue size (24 max) | path_queue[24] at FootClass+0x5E0 | MAX_PATH_SEGMENT_STEPS=24 in core.rs:45 | PASS |
| Final waypoint count (straight line) | 11 waypoints, no zig-zag | 11 waypoints, no zig-zag | PASS |
| No zig-zag on long flat move | Confirmed by above analysis | Confirmed by above analysis | PASS |

---

## 4. Disparities Found

### D1 — Pass 2 Drift-Detection Method (MEDIUM severity)

**gamemd.exe:** `Path_optimize_straight_segments` detects drift by tracking Chebyshev
distance regression — when `chebyshev < previous_best`, the path is "curving back."
This fires when the path turns away from a diagonal direction it had been tracking,
even if the cross-product drift is small.
Source: `PATH_SMOOTHING_AND_SPEED_RAMPING_GHIDRA_REPORT.md §3`.

**Our Rust:** `find_drift_segment` uses cross-product `|cum × ideal|² > dist² * DRIFT_THRESHOLD`.
This is a perpendicular drift measure, not a Chebyshev regression measure.

**Player impact:** For the straight-south scenario, both methods produce identical output
(no drift triggered). For diagonal-dominant paths (e.g. NE run that briefly jogs SE),
the Chebyshev method may trigger rerouting where our cross-product does not, or vice
versa. Player sees unexpected micro-turns preserved on diagonal routes.
**Trigger frequency:** Every diagonal move with any cardinal correction step — common
for non-axis-aligned orders.
File: `/src/sim/pathfinding/path_smooth.rs:361-407` (`find_drift_segment`).
gamemd evidence: `PATH_SMOOTHING_AND_SPEED_RAMPING_GHIDRA_REPORT.md §3`.

### D2 — Pass 2 Two-Ordering Retry (LOW-MEDIUM severity)

**gamemd.exe:** `FUN_0042be20` tries diagonal-first ordering; if it fails passability,
swaps to cardinal-first and tries again (`local_10 < 2` loop).
Source: `PATH_SMOOTHING_AND_SPEED_RAMPING_GHIDRA_REPORT.md §8.6`.

**Our Rust:** `reroute_segment` uses a Bresenham interleave ratio (`diag_remaining /
card_remaining` comparison) — a single attempt with no retry on blocked orderings.
If the first interleave choice hits a blocked cell, it returns None without retrying
the other ordering.

**Player impact:** On terrain with scattered obstacles, a rerouting that would succeed
cardinal-first silently falls back to the original path. Player sees units take a
slight dogleg where gamemd's second attempt would have found the straight line.
**Trigger frequency:** Whenever path optimization detects drift AND the optimal straight
path partially clips an obstacle. Low in open flat terrain, moderate on cluttered maps.
File: `/src/sim/pathfinding/path_smooth.rs:411-503` (`reroute_segment`).
gamemd evidence: `PATH_SMOOTHING_AND_SPEED_RAMPING_GHIDRA_REPORT.md §8.6`.

### D3 — Pass 2 Slope Validation Not Implemented (LOW severity for flat terrain)

**gamemd.exe:** `FUN_0042be20` calls `FUN_0056bcd0` (slope cost lookup at 4-cell
resolution) and `FUN_004dc760` (unit speed factor), rejecting reroutes that hit
> 3 steep cells (end-of-scan) or any steep cell (mid-scan).
Source: `PATH_SMOOTHING_AND_SPEED_RAMPING_GHIDRA_REPORT.md §8.4, §8.5`.

**Our Rust:** `reroute_segment` only calls `walkable(x, y)` — a terrain-blocked check.
No slope cost lookup, no steep-cell counting. Slope checking is not applicable to
the straight-south flat scenario, but is a gap on hilly maps.
**Trigger frequency:** Any map with terrain height variation where a reroute crosses
slope cells. Zero impact for the scenario traced here.
File: `/src/sim/pathfinding/path_smooth.rs:411-503`.
gamemd evidence: `PATH_SMOOTHING_AND_SPEED_RAMPING_GHIDRA_REPORT.md §8.4`.

---

## 5. Key Finding — Straight-Line Scenario Correct

**For the exact scenario (50,50)→(50,60) flat open grass):**
Both gamemd.exe and our engine produce an **identical straight-line path** with
11 waypoints and no zig-zag. The player sees no difference on this move.

The disparities above only manifest on diagonal moves with partial obstacles (D1, D2)
or hilly terrain (D3). The narrow straight-south scenario is PASS end-to-end.

---

## 6. Adjacent Findings (Out of Scope — Log Only)

- **Speed ramping** (out of scope per slot constraints): `DriveLocomotionClass` speed
  fields and `Process_Drive_Track` not traced here. See `PATH_SMOOTHING_AND_SPEED_RAMPING_GHIDRA_REPORT.md §5-6`.
- **Preview line rendering** (out of scope): The cursor hover path preview is drawn
  in the UI layer, not yet traced. Unknown if it uses the same smoothed path or raw A*.
- **Pass 1: slope validation** not in gamemd's `Path_smooth_single_segment` (0x42b420)
  but our Pass 1 has no slope check either — parity preserved on this missing feature.
- **Zone precheck**: confirmed active before A* (`PATH_SMOOTHING_AND_SPEED_RAMPING_GHIDRA_REPORT.md §8.1`).
  For flat open terrain, source and dest share same zone — precheck trivially passes both in
  gamemd and our `zone_search`.

---

## Sources

- `ra2-rust-game-docs/PATH_SMOOTHING_AND_SPEED_RAMPING_GHIDRA_REPORT.md` (primary)
- `ra2-rust-game-docs/PATHFINDING_ASTAR_GHIDRA_REPORT.md`
- `src/sim/pathfinding/path_smooth.rs`
- `src/sim/pathfinding/core.rs`
- `src/sim/movement/movement_path.rs`
