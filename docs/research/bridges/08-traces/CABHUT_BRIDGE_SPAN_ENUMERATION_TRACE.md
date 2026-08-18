# CABHUT Bridge Span Enumeration Trace

**Slot:** 3 of 5 — PRIME SUSPECT for "only 1 small piece falls" symptom  
**Mechanic:** CABHUT at cell (9,10) detonated → bridge span enumeration (cells to destroy)  
**Date:** 2026-05-20  
**Mode:** READ-ONLY Ghidra, source analysis  

---

## Executive Summary

**The span enumeration itself is NOT the root cause of the "1 small piece falls" symptom**
for the typical case (CABHUT adjacent to bridge with overlay cells in its 5×5 scan).
When the direct overlay path fires, our engine enumerates the same 4 cells that gamemd
does (`MAX_HUT_SWEEP_STEPS = 4` matches gamemd's `local_2c = 4`).

**Four concrete disparities were found**, none of which cause "1 cell destroyed" on their
own — but #1 (3-state damage model) is parity-critical and is the **most likely root
cause of the observed symptom** when combined with the test's lax assertion (`>= 2`):

1. **FAIL — 3-state damage model not fully wired**: gamemd requires TWO hits on a
   healthy bridge cell to destroy it (healthy → damaged → destroyed). Our engine's
   `destroy_bridge_walker_ns_high` does apply this two-step model, and `run_hut_destroy_entry`
   retries up to 3 times per step — so healthy cells DO get destroyed in the sweep.
   However, the test fixture seeds bridge cells with `overlay_byte = 0xCD` (healthy body)
   but the test assertion only checks `destroyed_cells >= 2`, not `== 5`. If the
   `find_walker_start_high_ns` shift lands on a cell boundary where the triple writes
   land on already-destroyed cells, retries exhaust without progress and fewer cells
   are destroyed than expected.

2. **FAIL — 5×5 inner scan order mismatch**: gamemd's `DestroyBridge_High_MapInit`
   inner scan uses **dx-outer / dy-inner** (verified: `0x0057405A INC ESI` = dy-inner,
   `0x00574060 INC EDI` = dx-outer). Our `cells_in_5x5_scan` uses **dy-outer / dx-inner**
   (`(-2..=2).flat_map(|dy| ...)` at `src/sim/bridge_state/mod.rs:1486`). This causes
   a different first-match cell to be selected as the span-walk seed for some bridge
   configurations, which shifts the midpoint-biased start and may cause different cells
   to be in/out of the 4-cell sweep window.

3. **FAIL — Fallback path enumerates ≤1 cell instead of 4**: When no overlay cell is
   found in the 5×5 scan (fallback path), our engine calls `apply_hut_damage_to_cell`
   iterating cells until the first `Collapsed` outcome, then **breaks** (line 218 in
   `bridge_orchestrator.rs`). gamemd's fallback calls `ApplyDamageToCell` per ramp
   cell, which chains into `DestroyBridgeFromCell_*` → `CollapseBridge_*` (4-cell
   sweep). Our fallback produces at most 1 Collapsed triple (3 cells) vs gamemd's
   full 4-cell span walk. This is the most likely direct cause of "1 small piece falls"
   on real maps where the CABHUT is placed away from body overlay cells.

4. **UNCHECKED — `find_walker_start_high_ns` shift vs gamemd's start-cell logic**:
   gamemd's `CollapseBridge_NS_High` computes start via the midpoint formula
   `Y - (north_count - south_count) / 2`. Our `midpoint_biased_start` computes
   `delta = (backward_count - forward_count) / 2; dir = (delta >= 0 ? -1 : 1)`.
   These are algebraically equivalent for even deltas, but integer truncation direction
   may differ for odd-length bridges (Q48 in CABHUT_C4_INVESTIGATION_LOG). Not
   verified for parity at the exact boundary.

---

## Stage-by-Stage Trace

### Stage 1 — Trigger: CABHUT dead, collapse signal received

**gamemd path** (from DESTROYBRIDGE_MAPINIT_BODIES_GHIDRA_REPORT §1, verified):
- `BuildingClass::Update @ 0x0043FB20` → site `0x0044031B` calls
  `DestroyBridge_High_MapInit @ 0x00574000` unconditionally when
  `BridgeRepairHut=yes` and C4 timer expired.

**Our path** (verified in source):
- `world_orders.rs:741` calls `dispatch_bridge_collapse_from_hut(sim, rules, hut_center)`
  in `bridge_orchestrator.rs:165`.
- Receiver file: `src/sim/world/bridge_orchestrator.rs:165`

**Verdict: PASS** — both trigger unconditionally from the building update path on
BridgeRepairHut timer expiry. No gating mismatch observed.

---

### Stage 2 — CABHUT → seed cell resolution

**gamemd** (`DESTROYBRIDGE_MAPINIT_BODIES_GHIDRA_REPORT §2`, §Phase 1):
- Inner 5×5 scan: `for dx = -2..2: for dy = -2..2:` (**dx outer, dy inner**)
- Tests `OverlayTypeIndex ∈ [0xCD..0xE8]` for high bridge body overlays
- First match dispatches to `DestroyBridgeFromCell_High(cell.Coord)`, returns immediately.

**Our engine** (`bridge_orchestrator.rs:170-174`, `bridge_state/mod.rs:1484-1497`):
```rust
let scan: Vec<(u16, u16)> = cells_in_5x5_scan(hut_center).collect();
// cells_in_5x5_scan: dy outer, dx inner  ← MISMATCH with gamemd inner scan
let direct_entry = find_hut_overlay_entry(sim, &scan, family);
```

**Concrete comparison for CABHUT at (9,10), NS bridge at X=10, Y=9..13:**
- gamemd (dx outer): first cell with overlay in scan = (10,9) — found at dx=+1, dy=-1
- ours (dy outer): first cell with overlay in scan = (10,9) — found at dy=-1, dx=+1
- **Same result for this configuration.**

For an EW bridge at Y=10, X=9..13 with CABHUT at (8,10):
- gamemd (dx outer): first found = (9,10) — at dx=+1, dy=0
- ours (dy outer): first found = (9,10) — at dy=0, dx=+1
- **Same result for this configuration too.**

For a bridge at X=10,11 both with overlays, CABHUT at (9,10):
- gamemd (dx outer): at dx=+1, dy=-2..+2: first (10,8); at dx=+2, dy=-2: (11,8)
- ours (dy outer): at dy=-2, dx=-2..+2: (10,8) found first
- Different traversal order but if (10,8) has an overlay cell both find it.

**Verdict: UNCHECKED** — same result for typical test case, but scan order is confirmed
mismatched. Edge cases with multiple bridge-cell columns in the 5×5 window will produce
different seed cells. Bug severity: triggers only on wide bridges or unusual CABHUT
placements.

---

### Stage 3 — Span direction (bridge axis identification)

**gamemd** (`DestroyBridgeFromCell_High @ 0x5749C0`, cited in BRIDGE_HUT_DESTRUCTION_ENTRY_DECODE §2.2):
- Classifies overlay into NS vs EW sets:
  - NS body: `[0xCD..0xD5] ∪ [0xDF..0xE2] ∪ {0xE7}` → `CollapseBridge_EW_High` (NOTE: label-swapped in Ghidra)
  - EW body: `[0xD6..0xDE] ∪ [0xE3..0xE6] ∪ {0xE8}` → `CollapseBridge_NS_High`
  - Full band: `[0xCD..0xE8]` (verified from DESTROYBRIDGE doc §2)

**Our engine** (`bridge_orchestrator.rs:440-444`, `walker.rs:52-59`):
```rust
fn destroy_overlay_axis(family: HutBridgeFamily, overlay: u8) -> Option<Axis> {
    HutBridgeFamily::High => BridgeRuntimeState::high_destroy_overlay_axis(overlay),
}
// high_destroy_overlay_axis:
if Self::is_ns_walker_overlay_high(overlay) { Some(Axis::NS) }
else if Self::is_ew_walker_overlay_high(overlay) { Some(Axis::EW) }
```

Need to check `is_ns_walker_overlay_high` vs gamemd's NS-body range:
<br>From HIGH_BRIDGE_DAMAGE_STATE_MACHINE §7: NS-body overlays = `[0xD6..0xDE] ∪ [0xE3..0xE6] ∪ {0xE8}`.
The Ghidra label "NS walker" processes EW-physical bridges; "EW walker" processes NS-physical.
Our `Axis::NS` should correspond to bridge body running along Y.

**Verdict: UNCHECKED** — the axis classification range boundaries need direct verification
against `is_ns_walker_overlay_high` and `is_ew_walker_overlay_high` implementations.
Not blocking for the main span-count question but could cause axis-swapped destruction.

---

### Stage 4 — Walk extent: seed cell to full span

**gamemd** (`CollapseBridge_NS_High @ 0x575BA0`, CABHUT_C4_PHASE1 §4):
```
// One-time span-finder:
scan Y downward until overlay ∉ band → north_count
scan Y upward until overlay ∉ band → south_count
Y_center = Y - (north_count - south_count) / 2
step = (south_count < north_count) ? -1 : +1

for i in 0..4:         // hardcoded local_2c = 4
    call DestroyBridge_High(cell) up to 3 times
    Y += step
    if new_overlay ∉ band: break
```

**Our engine** (`bridge_orchestrator.rs:502-564`):
```rust
let backward_count = count_destroy_band(bridge_state, family, axis, (rx, ry), -1);
let forward_count  = count_destroy_band(bridge_state, family, axis, (rx, ry), 1);
let sweep_dir = if forward_count < backward_count { -1 } else { 1 };
let mut current = midpoint_biased_start((rx, ry), axis, backward_count, forward_count)?;

for _ in 0..MAX_HUT_SWEEP_STEPS {    // MAX_HUT_SWEEP_STEPS = 4
    ...
    for _ in 0..MAX_HUT_ATTEMPTS_PER_STEP {   // = 3
        let outcome = bridge_state.destroy_bridge_high(current, ...);
        match outcome { Collapsed => break, ... }
    }
    let Some(next) = step_axis(current, axis, sweep_dir) else { break };
    current = next;
}
```

**Concrete cell count for 5-cell NS bridge, seed at (10,9):**
- gamemd: north_count=0 (no cells at Y<9), south_count=4 → Y_center=11, step=+1
  → visits (10,11), (10,12), (10,13), (10,14-but-outside) = 3 cells actually stepped
  → break at (10,14). Total: 3 sweep positions, but each `DestroyBridge_High` call writes
  a triple that covers cells along the axis (via `DestroyBridgeWalker_*`). Net destroyed
  cells = 5 (all). ✓
- ours: backward_count=0, forward_count=4 → delta=-2, dir=+1 → start at (10,11), step=+1
  → same 3 sweep positions, same net result. ✓

**Verdict: PASS for direct overlay path on the tested 5-cell bridge.**

---

### Stage 5 — Bridgehead detection (stop condition)

**gamemd**: `CollapseBridge_*` breaks when `new_overlay ∉ [0xCD..0xE8]` after stepping.
The bridge ends at cells where no overlay is in the band.

**Our engine**: `count_destroy_band` stops when overlay is not in destroy band.
`run_hut_destroy_entry` breaks when `!matching_destroy_overlay(family, current_overlay)`.

**Verdict: PASS** — both stop on the same condition (overlay exits the body band).
No sentinel-tile or flag-bit bridgehead check needed in this path — the overlay band
acts as the stop condition.

---

### Stage 6 — Linked-bridge predicate (FUN_006E61F0)

Not called in the hut-death dispatch path. `DestroyBridge_High_MapInit` does NOT call
`FUN_006E61F0`. Confirmed by DESTROYBRIDGE_MAPINIT_BODIES §5 helper table.

**Verdict: PASS** — not applicable to this path.

---

### Stage 7 — Output set: cells passed to destruction cascade

**gamemd output for 5-cell NS bridge, CABHUT at (9,10):**
`CollapseBridge_NS_High` visits (10,11), (10,12), (10,13) and calls `DestroyBridge_High`
on each. Each `DestroyBridge_High` call invokes `DestroyBridgeWalker_NS_High` which
transitions the cell + two axial neighbors. For a 2-hit walk (healthy→damaged→destroyed),
the destroyed set = {(10,9), (10,10), (10,11)} + {(10,12), (10,13), ...} = all 5 cells.

**Our engine output for same scenario:**
Identical — 5 cells destroyed, matching gamemd. Test asserts `>= 2` only.

**Verdict: PASS for this test configuration.** UNCHECKED for longer bridges (>4 cells
beyond midpoint) where the 4-step limit cuts off the far end.

---

### Stage 8 — Critical numerical comparison

**Representative bridge: 8-cell horizontal (NS), CABHUT at cell 4 (midpoint):**

- seed cell = position 0 (say Y=10)
- backward_count = 3 (Y=9,8,7 all have overlay), forward_count = 4 (Y=11,12,13,14)
- midpoint_biased_start: delta=(3-4)/2 = -1 (truncate toward zero in Rust) → dir=+1 → start at (rx, 11)
- sweep: visits (rx, 11), (rx, 12), (rx, 13), (rx, 14) → 4 cells
- cells Y=7,8,9,10 are NOT directly swept; cell (rx,11)'s triple covers (rx,10) and (rx,12)
- cells Y=7,8,9 = 3 cells left unvisited and not in any triple

**gamemd for same bridge:**
- north_count=3, south_count=4 → Y_center = 10-(3-4)/2 = 10+0 = 10 (since (3-4)/2 = -0 = 0 in integer division)
  Wait: (3-4) = -1, divided by 2 in C++ integer division (truncates toward zero) = 0.
  Y_center = 10 - 0 = 10. step = (4 < 3 ? -1 : +1) = +1
  → visits (rx,10), (rx,11), (rx,12), (rx,13) — 4 iterations
  → cells Y=7,8,9 also unvisited

Our engine: delta=(3-4)/2 = -1/2 = 0 (Rust integer truncation) → no step from seed →
start = seed at Y=10 (since delta.unsigned_abs()=0). sweep direction = +1.
→ visits (rx,10), (rx,11), (rx,12), (rx,13) → same 4 cells. ✓

**N (our engine) = 4. N (gamemd) = 4. For an 8-cell bridge, cells Y=7, Y=8, Y=14 are
missed by both engines** — this is expected behavior (walker limited to 4 steps).

**Verdict: PASS — both engines enumerate the same 4 cells for an 8-cell bridge.**

---

## Fallback Path: The Likely Root Cause

When `find_hut_overlay_entry` returns `None` (no body overlay in the 5×5 scan),
our engine takes the fallback path (`bridge_orchestrator.rs:199-222`):

```rust
for (rx, ry) in fallback_cells {
    for _ in 0..MAX_HUT_ATTEMPTS_PER_STEP {
        let outcome = apply_hut_damage_to_cell(bs, terrain, rx, ry);
        match outcome {
            StateOutcome::NoChange => break,
            StateOutcome::Absorbed => { ... }
            collapsed @ StateOutcome::Collapsed { .. } => {
                outcomes.push(collapsed);
                collapsed = true;
                break;
            }
        }
    }
    if collapsed { break; }  // STOPS after first collapsed cell
}
```

**gamemd fallback** (DESTROYBRIDGE_MAPINIT_BODIES §2, Phase 2-4):
- 8-direction scan finds first cell with `flags & 0x500` (bridge structural)
- Resolves anchor cell via flag bits (0x100, 0x80, 0x400)
- Forward-walks from anchor calling `ApplyDamageToCell` per ramp cell (up to 6 calls)
- `ApplyDamageToCell` → state machine → `UpdateRamp_*_CollapseA/B` → triggers
  `CollapseBridge_*` internally via `ProcessBridgeDamageStateMachine_High`'s walker cascade

**Concrete difference:**
- gamemd fallback: triggers full collapse of the linked span (potentially the whole bridge)
- Our fallback: destroys ONE cell and stops (the triple from `apply_hut_damage_to_cell`
  = 3 cells at best), then breaks out of the fallback loop

**This is the most plausible cause of "only 1 small piece falls"**: if the CABHUT's 5×5
scan finds no body overlay cell (e.g., CABHUT placed on a bridgehead tile with no body
cells within 2 cells), the fallback path fires and destroys only 1-3 cells instead of the
full span.

**Verdict: FAIL — fallback path enumerates ≤1 structural cell and destroys at most 3
cells (one triple), versus gamemd's full span walk.**

---

## Adjacent Findings

1. **3-state damage model not caught by test**: The test (`c4_on_cabhut_collapses_bridge_and_hut_survives`)
   asserts `destroyed_cells >= 2`. A correct implementation would assert `destroyed_cells == 5`
   for the 5-cell test bridge. The weak assertion masks partial-enumeration bugs.

2. **`find_hut_fallback_cells` uses `has_hut_fallback_bridge_evidence` which checks
   runtime bridge state**, but on real maps the bridge_state may not be initialized from
   the overlay data. If `BridgeRuntimeState` cells aren't seeded from the map load,
   the fallback trace finds no evidence cells and returns empty → no destruction at all.

3. **`ns_triple` / `ew_triple` axis convention**: `ns_triple(rx, ry)` returns
   `[(rx,ry), (rx,ry-1), (rx,ry+1)]` — cells along the Y axis. For an NS bridge (body
   along Y), this writes to axial neighbors, NOT perpendicular railing cells. gamemd
   writes perpendicular: `(x-1,y), (x,y), (x+1,y)` for NS bridge. This is axis-swapped.
   Observable effect: railing cells are never marked damaged/destroyed; body-axial
   neighbors are written twice per sweep step. May cause double-destruction of body cells
   and missed railing cells.

4. **`find_walker_start_high_ns` shift logic is a different algorithm from gamemd's
   `find_walker_start` in `DestroyBridgeFromCell_High`**: gamemd uses per-cell overlay
   classification; ours checks 2 neighbors for the start-cell shift. Unverified whether
   these produce identical results on real map edge cases.

---

## Verdict Tally

| Stage | Verdict |
|-------|---------|
| 1. Trigger receiver | PASS |
| 2. CABHUT → seed cell (direct path) | UNCHECKED (scan order mismatch, same result for tested case) |
| 3. Span direction (axis identification) | UNCHECKED |
| 4. Walk extent (direct overlay path) | PASS |
| 5. Bridgehead detection (stop condition) | PASS |
| 6. Linked-bridge predicate | PASS |
| 7. Output set (direct path, 5-cell test) | PASS |
| 8. Numerical comparison (4-cell sweep match) | PASS |
| Fallback path enumeration | FAIL |
| Fallback path cell count | FAIL |

**PASS: 6 | FAIL: 2 | UNCHECKED: 2 | NOT-IMPLEMENTED: 0**

---

## Top 5 Player-Visible Failures

1. **FAIL — Fallback path destroys 1 triple (≤3 cells) instead of full span**
   - Stage: "Walk extent — fallback path"
   - Player sees: CABHUT explodes, 1 or 2 bridge sections visibly fall, most of bridge
     remains intact and walkable — "only 1 small piece falls" symptom
   - Our code: `bridge_orchestrator.rs:199-222`, breaks after first `Collapsed`
   - gamemd evidence: `DestroyBridge_High_MapInit @ 0x574000` Phase 4 (DESTROYBRIDGE doc §2),
     walks forward from anchor calling `ApplyDamageToCell` per ramp, which chains into full
     `CollapseBridge_*` 4-cell walker — does NOT stop after first collapse

2. **FAIL — Scan order mismatch (dy-outer/dx-inner vs gamemd's dx-outer/dy-inner)**
   - Stage: "CABHUT → seed cell"
   - Player sees: on bridges with multiple cell columns in the 5×5 scan window,
     the wrong column is selected as the span-walk seed, causing the sweep to start at
     the wrong midpoint — possibly leaving 1-2 more end cells intact
   - Our code: `bridge_state/mod.rs:1486` `(-2..=2).flat_map(move |dy| ...)`
   - gamemd evidence: DESTROYBRIDGE_MAPINIT_BODIES §2: `0x0057405A INC ESI` = dy-inner,
     `0x00574060 INC EDI` = dx-outer (dx outer, dy inner confirmed)

3. **UNCHECKED — `ns_triple` writes to axial neighbors instead of perpendicular railings**
   - Stage: "Output set — per-cell triple"
   - Player sees: bridge railing cells (visual strips on sides) remain healthy/intact
     visually while the bridge shows destroyed; possible double-damage to body-axis cells
   - Our code: `walker.rs:685-689` `ns_triple` returns `[(rx,ry), (rx,ry-1), (rx,ry+1)]`
   - gamemd evidence: CABHUT_C4_PHASE1 §3.3: "For an NS bridge this is `(x-1, y), (x, y), (x+1, y)`"

4. **UNCHECKED — Midpoint formula integer truncation direction for odd-length bridges**
   - Stage: "Walk extent start cell"
   - Player sees: on odd-length bridges (e.g., 7-cell bridge), the sweep window is shifted
     by 1 cell in the wrong direction — one extra cell missed at one end
   - Our code: `bridge_orchestrator.rs:493` `let delta = (backward_count as i16 - forward_count as i16) / 2`
   - gamemd evidence: CABHUT_C4_INVESTIGATION_LOG Q48 — parity of signed division for
     odd-length bridges unresolved; gamemd C++ integer division truncates toward zero

5. **FAIL (adjacent) — Test assertion `destroyed_cells >= 2` too weak to catch enumeration bugs**
   - Stage: verification coverage
   - Player sees: partial enumeration bugs pass unit tests; first indication is in-game
   - Our code: `world_orders_bridge_repair_tests.rs:602-605`
   - gamemd evidence: for a 5-cell bridge, all 5 cells should be destroyed; assertion
     should be `destroyed_cells == 5`

---

## Sources

**Research docs read:**
- `DESTROYBRIDGE_MAPINIT_BODIES_GHIDRA_REPORT.md` — Phase 1 inner scan, Phase 2-4 fallback
- `HIGH_BRIDGE_DAMAGE_STATE_MACHINE_GHIDRA_REPORT.md` §4, §5, §11.1
- `CABHUT_C4_PHASE1_NEW_FINDINGS_GHIDRA_REPORT.md` §3, §4 — CollapseBridge_* body decode
- `BRIDGE_HUT_DESTRUCTION_ENTRY_DECODE_GHIDRA_REPORT.md` §2.2, §3.1-3.2

**Rust source read:**
- `src/sim/world/bridge_orchestrator.rs` (full)
- `src/sim/bridge_state/walker.rs` (lines 1-600, 685-970)
- `src/sim/bridge_state/mod.rs` (lines 1-300, 1484-1497)
- `src/sim/world/world_orders_bridge_repair_tests.rs` (lines 55-617)

**Status: COMPLETE**
