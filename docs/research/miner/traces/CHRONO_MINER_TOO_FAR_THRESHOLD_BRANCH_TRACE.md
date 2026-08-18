# Trace: ChronoHarvTooFarDistance Threshold Branch — Drive vs Warp on Inbound

**Mechanic:** Chrono miner inbound distance check in `UnitClass::Mission_Harvest` state 2.
**Scope:** Close branch (drive), Far branch (warp), edge cases (operator, units).
**Date:** 2026-05-20
**Ghidra:** OFFLINE — all claims sourced from existing docs.

---

## 1. gamemd Ground Truth

**Source:** `CHRONO_MINER_TELEPORT_GHIDRA_REPORT.md` §14 and §16.

gamemd's `UnitClass::Mission_Harvest` (0x73E5E0), state 2 (RETURN), at address 0x73EE42:

```c
// Chrono miner path (Teleporter=yes):
dist = 3D_Euclidean_Distance(unit, dock);  // 3D Euclidean in leptons
                                            // 'dock' = BuildingClass* from Find_Docking_Bay
if (dist <= ChronoHarvTooFarDistance * 0x100) {
    // DRIVE: radio refinery, reserve dock slot, advance to state 3
} else {
    // WARP: compute dock-adjacent cell, call Set_Destination (empty cell),
    //       TeleportLoco fires via Head_To_Coord
}
```

**Critical specifics (from §14, §16):**
- Operator: `<=` (less-than-or-equal), verified at 0x73EE42.
- Unit: leptons. `0x100 = 256` leptons per cell.
- Threshold source: `RulesClass+0xD7C` = `ChronoHarvTooFarDistance`, default **50 cells**.
- Distance point on building: `dock.Location` = **origin cell** (top-left) of the refinery
  `BuildingClass` returned by `Find_Docking_Bay`. Not center, not queue cell.
- Distance formula: `(int)sqrt(dx*dx + dy*dy + dz*dz)` — Euclidean, integer-cast (floor).
  On a flat map, `dz=0` so it reduces to 2D Euclidean.

**Drive at exactly threshold:** `dist == 50 * 256 = 12800` → `12800 <= 12800` → TRUE → **drive**.
**Warp at threshold+1 lepton:** `dist == 12801` → `12801 <= 12800` → FALSE → **warp**.

---

## 2. Our Implementation

**File:** `src/sim/miner/miner_system.rs`

### Comparison operator (all three call sites):
```rust
// begin_return (line 789), handle_return (line 527), handle_forced_return (line 618):
let threshold = config.too_far_threshold_chrono as u32;
let far_enough = cell_dist_sq((snap.rx, snap.ry), center) > threshold * threshold;
```

### Distance function (line 1205):
```rust
fn cell_dist_sq(a: (u16, u16), b: (u16, u16)) -> u32 {
    let dx = a.0 as i32 - b.0 as i32;
    let dy = a.1 as i32 - b.1 as i32;
    (dx * dx + dy * dy) as u32
}
```

### Threshold source (mod.rs line 228):
```rust
too_far_threshold_chrono: general.chrono_harv_too_far_distance.max(1) as u16,
```
Default in `GeneralRules::default()` (ruleset.rs line 575): `chrono_harv_too_far_distance: 50`.
INI parsing (ruleset.rs line 936): `general.get_i32("ChronoHarvTooFarDistance").unwrap_or(50)`.

### Reference point (line 787, 525, 616):
```rust
let center = refinery_center_cell_for_sid(sim, rules, rsid).unwrap_or(dock);
```
```rust
fn refinery_center_cell_for_sid(...) -> Option<(u16, u16)> {
    // line 993:
    Some((entity.position.rx + w / 2, entity.position.ry + h / 2))
}
```
For a 4×3 GAREFN at (10,10): center = (10+2, 10+1) = **(12, 11)**.
gamemd origin: **(10, 10)**.

---

## 3. Sub-scenario (a): CLOSE Branch

**Setup:** Refinery GAREFN at (10,10), foundation 4×3.
- gamemd origin cell: (10, 10).
- Our center cell: (12, 11).
- Miner at (15, 11).

**gamemd computation:**
- dx = (15 - 10) × 256 = 1280 leptons (X), dy = (11 - 10) × 256 = 256 leptons (Y).
- dist = sqrt(1280² + 256²) = sqrt(1638400 + 65536) = sqrt(1703936) ≈ **1305.3** → floor = 1305.
- threshold = 50 × 256 = 12800.
- 1305 <= 12800 → TRUE → **DRIVE**. Correct.

**Our computation:**
- cell_dist_sq((15,11), (12,11)) = (15-12)² + (11-11)² = 9 + 0 = 9.
- threshold² = 50² = 2500.
- 9 > 2500 → FALSE → **DRIVE** (drive falls through). Correct.

**Verdict:** PASS — both resolve to drive. The reference-point difference (center vs origin) does
not affect this case: 4 cells from origin, 3 cells from center, both far below 50 cells.

**Drive path check:** After the close branch, `begin_return` sets `state = ReturnToRefinery` with
no teleport issued. `handle_return` then calls `issue_move_if_idle` targeting `dock` (the queue
cell from `refinery_dock_for_sid`). For GAREFN 4×3 at (10,10): queue cell = (14, 11).
Miner is at (15, 11), 1 cell east of queue cell. `issue_move_if_idle` issues A* to (14, 11).
Path = ~1 cell west. **PASS** for observable drive-path output.

---

## 4. Sub-scenario (b): FAR Branch

**Setup:** Refinery GAREFN at (10,10), miner at (62, 11).
**Euclidean distances:**
- To gamemd origin (10, 10): dx=52, dy=1. dist_cells = sqrt(52² + 1²) ≈ 52.01.
- To our center (12, 11): dx=50, dy=0. dist_cells = 50.00 exactly.

**gamemd computation (leptons):**
- dx = 52 × 256 = 13312, dy = 1 × 256 = 256.
- dist = sqrt(13312² + 256²) = sqrt(177,209,344 + 65,536) = sqrt(177,274,880) ≈ 13314.5 → 13314.
- threshold = 50 × 256 = 12800.
- 13314 <= 12800 → FALSE → **WARP**. Correct.

**Our computation:**
- cell_dist_sq((62,11), (12,11)) = 50² + 0² = 2500.
- threshold² = 2500.
- 2500 > 2500 → FALSE (not strictly greater) → **DRIVE**.

**CRITICAL FINDING — FAR BRANCH BUG AT EXACTLY 50 CELLS (CENTER REFERENCE):**
When the miner is at (62, 11) and the refinery is at (10, 10), our center is (12, 11).
Cell distance = 50 cells exactly. `cell_dist_sq = 2500 > 2500` is **false** → our code resolves
to DRIVE. gamemd at 13314 leptons > 12800 → resolves to **WARP**. The miner should warp but drives.
This is a boundary-point discrepancy caused by the reference-point mismatch (center vs origin).

**General FAR branch (miner truly >50 cells from both points, e.g., (70, 11)):**
- Our center: cell_dist_sq((70,11),(12,11)) = 58² = 3364 > 2500 → **WARP**. Correct.
- Warp fires: `spawn_warp_effects` + `issue_teleport_command(..., is_harvester=true)`.
  WarpOut anim spawned at departure AND arrival (two WorldEffect entries). ChronoOutSound at
  departure, ChronoInSound at arrival. Teleport resolves in 1 tick (is_harvester=true path).
  Miner snaps to queue cell (14, 11). **PASS** for unambiguously-far scenario.

---

## 5. Edge Cases at Threshold — Operator and Unit Analysis

### 5a. Operator (`<=` vs `>`)

**gamemd:** `dist <= threshold * 256` → drive at exactly 12800 leptons, warp at 12801.
**Our code:** `cell_dist_sq > threshold²` → drive when `cell_dist_sq <= threshold²`.

In cell space: drive when `dist_cells_sq <= threshold²`, warp when `dist_cells_sq > threshold²`.
This means: drive at exactly `threshold` cells, warp strictly beyond. **The logical operator
matches gamemd's `<=` for drive / `>` for warp, but only when applied to the same reference
point and the same distance metric.**

**Verdict:** Operator direction is CORRECT (both engines drive at-exactly-threshold, warp beyond).

### 5b. Unit Conversion (cells vs leptons)

gamemd: `lepton_dist <= threshold_cells × 256`
Squaring both sides: `lepton_dist² <= threshold_cells² × 65536`
Equivalently in cells: `cell_dist² × 65536 <= threshold_cells² × 65536`
→ `cell_dist² <= threshold_cells²`

Our code: `cell_dist_sq <= threshold²` (for drive case).

**The unit conversion is mathematically equivalent provided the distance is computed on the
same grid point.** The issue is NOT the cell-vs-lepton translation — that algebra is sound.
The issue is the reference point divergence documented below.

### 5c. Euclidean vs Manhattan vs Chebyshev

**gamemd:** 3D Euclidean (`sqrt(dx² + dy² + dz²)`), which on flat terrain reduces to 2D Euclidean.
**Our code:** 2D Euclidean via `cell_dist_sq`. On flat terrain: **MATCH**.

### 5d. Reference Point Mismatch (FAIL)

**gamemd:** Measures from miner to `dock.Location` = **building origin cell** (top-left).
For GAREFN 4×3 at (10,10): origin = (10, 10).

**Our code:** Measures from miner to `refinery_center_cell_for_sid` = `(rx + w/2, ry + h/2)`.
For GAREFN 4×3 at (10,10): center = (12, 11).

**Offset from true reference:** 2 cells east, 1 cell south of origin.

**Quantitative effect:** The effective boundary shifts by ~2.2 cells (Euclidean distance from
origin to center of a 4×3 building). Miners in the arc that is 48–52 cells from the building
origin but 50 cells from the center will be misclassified:
- 50 cells from center, 52.2 cells from origin → gamemd says warp, we say drive.
- Triggers every match cycle when a fully-loaded chrono miner is at the boundary zone.
- Observable: miner drives when it should warp (slower return, no warp visual/sound, visible
  timing difference on the return trip).

---

## 6. Warp Sequence Verification (FAR Branch, Unambiguously Far)

When `far_enough = true`, our code path:

1. `spawn_warp_effects(sim, rules, type_id, depart, arrive)` — spawns two `WorldEffect`
   entries (WarpOut anim at departure + arrival). Also emits `ChronoOutSound` / `ChronoInSound`
   sound events. **Matches gamemd §5 Phase 0 steps 3 and 15 (WarpAway at both endpoints).**
   Note: our code uses `warp_out` from `rules.general.warp_out.name`. gamemd spawns WarpAway
   (Rules+0x33C) at both departure AND arrival. **PASS** — same anim at both endpoints.

2. `issue_teleport_command(&mut sim.entities, snap.entity_id, dock, &rules.general, true)`
   — with `is_harvester=true` → instant 1-tick teleport, no chrono lock applied.
   **PASS** — matches §5: self-teleport resolves in 1 tick (Phase 0 one-shot).

3. Miner snaps to `dock` (queue cell, e.g., (14, 11) for GAREFN at (10,10)).
   **PASS** — destination is the queue cell adjacent to the refinery, not the building itself.

4. `teleport_state` guard in `handle_return` (line 507) holds the miner in
   `ReturnToRefinery` for the tick the teleport is materializing. Next tick,
   `is_adjacent_or_at((14,11), (14,11))` → true → transitions to `Dock` /
   `RefineryDockPhase::Approach`. **PASS** — correct state transition sequence.

---

## 7. Verdict Tally

PASS: 6 | FAIL: 2 | UNCHECKED: 0 | NOT-IMPLEMENTED: 0

---

## 8. Top 5 Player-Visible Failures

1. **FAIL — Reference-point mismatch on distance check.**
   Stage: `begin_return` / `handle_return` / `handle_forced_return`.
   Observable: Miner at boundary zone (48–52 cells from refinery origin) drives when it
   should warp. No warp anim, no warp sound, slower return trip — visible every match.
   File:line: `miner_system.rs:787`, `miner_system.rs:525`, `miner_system.rs:616`
   (`refinery_center_cell_for_sid` returns `(rx + w/2, ry + h/2)` instead of `(rx, ry)`).
   gamemd evidence: CHRONO_MINER_TELEPORT_GHIDRA_REPORT.md §14 —
   `dist = 3D_Euclidean_Distance(unit, dock)` where `dock` is the `BuildingClass*` whose
   `Location` is the **origin cell** of the refinery, not its geometric center.

2. **FAIL — Exact 50-cell (center-reference) case misclassified.**
   Stage: All three call sites, `cell_dist_sq > threshold * threshold`.
   Observable: Miner exactly 50 cells from refinery center (≈52 cells from origin) drives
   instead of warping. One specific ring of positions around every refinery is wrong.
   File:line: `miner_system.rs:789` — `2500 > 2500` evaluates false → drive.
   gamemd evidence: `dist <= 12800` at 52 cells → 52 × 256 = 13312 > 12800 → warp.
   The failure is a symptom of finding #1 (reference point), not a separate operator bug.

---

## 9. Status

COMPLETE.

All three call sites (`begin_return`, `handle_return`, `handle_forced_return`) contain the same
reference-point bug. The operator direction (`>` for warp, `<=` for drive) is correct. The unit
conversion (cells² vs leptons²) is algebraically equivalent. The warp-effects and teleport
command for the unambiguously-far case are correct. The single root bug is:

> **gamemd measures distance to the building's origin cell; we measure to its geometric center.**

Fix: replace `refinery_center_cell_for_sid` call with the building's `(entity.position.rx, entity.position.ry)` directly in all three call sites (or add a `refinery_origin_cell_for_sid` helper that returns the origin).
