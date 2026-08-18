# CheckBridgeTraversal Port — Pathfinding Height-Diff Legality

## Goal

Mirror gamemd's `CheckBridgeTraversal` (0x4D9C60) height-diff legality predicate inside our A* neighbor expansion: gate diff-1 transitions on the lower cell's SlopeIndex, and hard-block diff ∈ {±2, ±3, ±5+} (everything except diff-0 same-level and diff-±4 bridge entry/exit).

## Architecture Context

**Current state.** `ResolvedTerrainCell.slope_type` is already populated at map load from the TMP `ramp_type` byte (= gamemd's CellClass+0x11C SlopeIndex). The byte stops at the `PathGrid::from_resolved_terrain_with_bridges` boundary in [src/sim/pathfinding/core.rs:976–1025](../../src/sim/pathfinding/core.rs#L976-L1025) — `PathCell` carries only `ground_walkable, bridge_walkable, transition, ground_level, bridge_deck_level`. A* (`astar_search`, same file) has no height-diff legality gate. The only height-aware code is:

1. [`compute_neighbor_height`](../../src/sim/pathfinding/core.rs#L123-L153) — decides what Z a new node carries. Handles diff==4 Ground→Bridge entry (requires `transition` flag); other diffs fall through to `neighbor.ground_level`. No legality enforcement, just Z propagation.
2. The `CLIFF_COST_MULTIPLIER` (×4) at [core.rs:564–566](../../src/sim/pathfinding/core.rs#L564-L566) — pays a cost penalty when `current.height != neighbor_height`, but does not block.

The net effect: A* permits every height transition (including 2-cell-tall cliff faces) at ×4 cost.

**The gap.** Fidelity probe (`tests/bridge_pathfinding_g5_g6_fidelity_probe.rs`) found **2,038 trusted firing pairs across 21 retail RA2/YR maps**. gamemd blocks these via `CheckBridgeTraversal`; Rust currently permits them. Standouts: Lostlake.mmx (1,457/1,457 = 100% of its diff-1 pairs), EB1.mmx (166), Dustbowl (144).

**Where the gate fits.** The neighbor expansion at [core.rs:413–605](../../src/sim/pathfinding/core.rs#L413-L605) already runs per-direction passability, corner-cutting, terrain-cost, and entity-soft-block checks. The legality gate is one more inline check in the same loop, placed after `compute_neighbor_height` and before the open-list push.

## Impact Analysis

**Files modified:**
- [src/sim/pathfinding/core.rs](../../src/sim/pathfinding/core.rs) — `PathCell` struct, `DEFAULT_WALKABLE_CELL`, `DEFAULT_BLOCKED_CELL`, `from_resolved_terrain_with_bridges`, `diff_cells`, `astar_search` neighbor loop.
- [src/sim/pathfinding/core_tests.rs](../../src/sim/pathfinding/core_tests.rs) — every `PathCell { ... }` literal (9 sites around lines 141–199, 221, 241; verify before editing).
- [src/sim/pathfinding/zone_map_tests.rs](../../src/sim/pathfinding/zone_map_tests.rs) — PathCell literals in test fixtures.
- [src/sim/bridge_state/mod.rs](../../src/sim/bridge_state/mod.rs), [src/sim/bridge_state/walker.rs](../../src/sim/bridge_state/walker.rs) — any PathCell construction in fixtures.

**Risk areas:**

- **Path-failure regression.** Any scenario that previously routed across a cliff edge (1,457 such cells on Lostlake alone) will now fail or detour. This is the parity win, but it'll surface as observably different unit behavior. Mitigation: the near-miss goal fallback at [core.rs:466](../../src/sim/pathfinding/core.rs#L466) already covers "goal cell unreachable"; intermediate cliff cuts force a detour. Add a focused regression test: unit ordered across a cliff edge routes the long way around via a ramp.
- **Determinism.** State hash changes for any sim scenario hitting a cliff edge. Replay-fixture tests and the state-hash determinism check (`tests/determinism_replay.rs`) need to be re-recorded if they happen to exercise cliff terrain. No new sources of nondeterminism (integer math, no floats).
- **Signed-byte heights.** gamemd uses `MOVSX` (signed i8) for `Level` reads. We use `u8`. Retail maps have `level ∈ [0, 15]` so unsigned and signed are equivalent. Negative-level malformed maps would diverge — accepted, not retail.
- **Backwards-compat.** PathCell layout grows by 1 byte (padded to 8). No save/replay format change. No public-API change.

**No** tick-ordering change, no new sim module, no new dependency.

## Chosen Approach

Add `slope_type: u8` to `PathCell`. Populate from `ResolvedTerrainCell.slope_type` at PathGrid build. In A*'s neighbor expansion, after `compute_neighbor_height`, before the open-list push, apply:

```rust
let diff = neighbor_height as i16 - current.height as i16;
let lower_slope = if diff < 0 { neighbor_cell.slope_type } else { cur_cell.slope_type };
let legal = match diff.abs() {
    0 => true,                  // L6 — implicit via compute_neighbor_height
    1 => lower_slope != 0,      // L4 — gamemd: SlopeIndex==0 on lower → block
    _ => false,                 // L5 + L7 — diff 2/3/4/5+ all block in our model
};
if !legal { continue; }
```

**Why `_ => false` correctly covers diff-4.** `compute_neighbor_height` transforms legitimate bridge transitions into diff-0 from the legality gate's perspective: Case 3 success (ground→bridge entry) returns `bridge_deck_level` while parent_height already equals deck-equivalent height; Case 2 deck-to-deck returns deck_level with no change. The only way `diff.abs() == 4` reaches the legality gate is via Case 1 (neighbor not bridge), which means we're walking a non-bridge 4-tall cliff face — and gamemd's `CheckBridgeTraversal` blocks exactly that case via `else { return 7 }` when neither bridgehead-anchor flag combination matches. So `_ => false` reproduces gamemd's behavior for diff-4 cliffs while leaving bridge entries untouched.

**Side-effect on existing test.** `test_cliff_cost_detours_under_uniform_base` (`core_tests.rs:134`) constructs a level-4 cliff between two level-0 cells and asserts A* takes the flat detour rather than crossing the cliff. Under the current code the cliff is permitted at ×4 cost and the alt route wins on cost; under the new gate the cliff is hard-blocked and the alt route is the only option. Both outcomes satisfy the existing assertion (`!path.contains(&(1, 0))`), but the path-cost reasoning in the test's comment becomes stale and should be updated.

## Tiny-Detail Ledger

| # | Detail | Source | Lives in |
|---|---|---|---|
| L1 | Lower cell = smaller `Level`. `diff < 0` → dst lower; `diff > 0` → src lower. | doc §3.1 lines 127–131 | Inline `if diff < 0 { neighbor_cell } else { cur_cell }` |
| L2 | SlopeIndex = TMP `ramp_type` byte (CellClass+0x11C). | doc §3.1, §3.3 | Named `slope_type` field; offset not exposed |
| L3 | SlopeIndex==0 means cliff/fallback. Nonzero (1–20) is canonical ramp. | doc §3.3 (writes at 0x47D5F9, 0x47DB52) | `slope_type == 0` semantics propagate from `metadata_from_set_name` default |
| L4 | Diff-1 result: lower `SlopeIndex==0` → return 7 (impassable). | doc §3.1 lines 126–131 | `match 1 => lower_slope != 0` |
| L5 | Diff ∈ {±2, ±3, ±5+} → ALWAYS return 7. | doc §3.1 lines 148–150 | `_ => false` (covers diff-4 cliffs too — see L7) |
| L6 | Diff==0 flat-step guard: fires when `targetHeight != src.Level` AND NOT all of (src 0x100, src 0x200, dst 0x100). In our model, `compute_neighbor_height` keeps unit Z synced with cell state — divergent diff-0 case unreachable. | doc §3.1 lines 119–124 | Implicit via `compute_neighbor_height`. Empirical canary: a unit test asserts that a unit at deck-height stepping to a non-bridge cell produces diff != 0 (Case 1 returns ground_level, so `diff.abs()` is 4 not 0). The legality gate then blocks via `_ => false`. |
| L7 | Diff==4 bridge entry/exit: requires bridge anchor + body flags on the appropriate side. In our model, legitimate bridge transitions emerge as diff-0 to the legality gate via `compute_neighbor_height` Case 3 (returns deck_level matching parent_height). Any diff-4 reaching the legality gate must be Case 1 (neighbor not bridge) — i.e., a non-bridge cliff face — and is correctly blocked by `_ => false`. | doc §3.1 lines 133–146 | Pre-existing in `compute_neighbor_height` Case 3; residual diff-4 blocked by the new `_` arm |
| L8 | Signed MOVSX height read. | doc §3.1 "Tiny details" #1 | Accepted divergence on malformed maps |
| L9 | Direction-invariant: same predicate for all 8 A* directions; diagonals via corner-cutting. | doc §3.1 algorithm | Gate runs identically per direction; existing corner-cutting at core.rs:519–551 |
| L10 | gamemd reads SlopeIndex from LOWER cell only, exactly once per call. | doc §3.1 lines 127–131 | Exactly one `slope_type` read per neighbor in the inline gate |

## Design

### Components

**`PathCell` (extended).**
```rust
pub struct PathCell {
    pub ground_walkable: bool,
    pub bridge_walkable: bool,
    pub transition: bool,
    pub ground_level: u8,
    pub bridge_deck_level: u8,
    pub slope_type: u8,        // NEW — TMP ramp_type byte (== gamemd SlopeIndex)
}
```

**`DEFAULT_*_CELL` constants** get `slope_type: 0` (default = "no metadata = cliff", consistent with `metadata_from_set_name` default).

**`PathGrid::from_resolved_terrain_with_bridges`** copies `cell.slope_type` into the new field.

**`astar_search` neighbor expansion** — single inline gate as shown above.

**`diff_cells`** compares `slope_type` alongside the existing five fields (defensive — slope_type is static-from-map-load today, but if `recalc_overlay_passability` or a future RecalcAttributes equivalent ever mutates it, the differ catches it).

### Interfaces / Contracts

No public-API changes. `PathCell` is a `pub struct` whose fields are read by A* internals and constructed in tests; the new field is additive. Callers constructing `PathCell` literals (test fixtures only) must add the new field.

### Data Flow

1. **Map load.** `ResolvedTerrainGrid::build` populates `ResolvedTerrainCell.slope_type` from `TmpTile.ramp_type` (existing).
2. **PathGrid build.** `PathGrid::from_resolved_terrain_with_bridges` copies `slope_type` into `PathCell` (new).
3. **A* search.** `astar_search` reads `cur_cell.slope_type` and `neighbor_cell.slope_type` during neighbor expansion, applies the legality match, decides to push or `continue` (new).
4. **Bridge state change.** When `BridgeRuntimeState` changes (destruction/repair), PathGrid is rebuilt via `from_resolved_terrain_with_bridges` — `slope_type` is unchanged (bridge state doesn't affect terrain slope).

### Error Handling

The gate uses `continue` to drop a neighbor — same convention as terrain-cost=0 and corner-cutting. A* returns `None` when no path is found, unchanged.

Determinism: integer comparison only, no floats. Lockstep correctness preserved.

### Testing Strategy

**Unit tests** in `core_tests.rs` (extend the existing height/cliff section):

1. `diff_1_slope_zero_blocks_going_up` — A* refuses to step from flat cell (slope=0, level 0) to slope=0 level-1 neighbor.
2. `diff_1_slope_zero_blocks_going_down` — Same in reverse direction.
3. `diff_1_slope_nonzero_permits` — slope=2 lower cell permits the step.
4. `diff_2_always_blocks` — two adjacent cells with level diff 2, both slope=2 — A* still blocks.
5. `diff_3_always_blocks` — level diff 3.
6. `diff_5_always_blocks` — level diff 5.
7. `diff_4_bridge_entry_regression` — existing bridge-entry case still works (pre-existing test should still pass).
8. `diff_0_unit_z_remains_consistent` — debug-assert canary: place a unit at deck height, step into a non-bridge neighbor at the same `ground_level`. `compute_neighbor_height` should produce a non-zero diff (Case 1 returns neighbor.ground_level which is ≠ deck_height); the legality gate then sees diff==4 (legal exit) or diff==-4 / diff>=2 (blocked). Assert no diff-0 case with mismatched Z arises.
9. `cliff_detour_via_ramp` — integration: a flat cell at level 0, a ramp cell at level 0/slope=2 next to it, a flat cell at level 1 beyond the ramp, and a direct cliff diff-1 between the level-0 and level-1 flats. A* must route via the ramp, not via the cliff.

**Fidelity-probe re-run** (the existing `bridge_pathfinding_g5_g6_fidelity_probe`): after implementation, the **probe is no longer the validator**. The probe scans terrain only — it doesn't know if A* would actually take a step. The relevant post-impl validation is:

- Add an extension to the probe (or a sibling test) that, for each candidate firing pair, calls `astar_search` from one cell to the other and asserts the path is rejected (or routes around). Reuse the probe's per-map scaffolding.

**No new dependency** on retail map fixtures inside the main test suite — keep the heavy probe under `#[ignore]`.

## Architectural Decisions

**Patterns followed.**
- PathCell as the A*-hot cache: same pattern as `bridge_walkable`, `bridge_deck_level`. The cell is the canonical place for data A* needs every neighbor; new field joins the existing five.
- Inline gate in the neighbor loop: same pattern as `is_cell_passable_for_mover`, corner-cutting, terrain-cost=0 checks. No new helper function unless the match grows.

**Patterns deviated from.** None.

**Tech debt introduced.**
- L6 (diff-0 flat-step guard) is not an explicit line of code — it relies on `compute_neighbor_height` keeping unit Z in sync. Documented via a canary unit test that asserts the divergent-state case (unit at deck height stepping to a non-bridge cell) produces a non-zero diff. If the canary ever fails (refactor of compute_neighbor_height, new bridge mechanic, etc.), add an explicit guard. **Acceptable debt:** the alternative is a guard expressed in flag combinations our model doesn't directly mirror — harder to keep correct than a test that proves the divergent state is unreachable.

## Alternatives Considered

- **Approach B — look up slope from `AStarOptions.resolved_terrain` in the hot loop.** Rejected: hidden coupling — callers that don't wire `resolved_terrain` silently degrade the gate to "always permit". Determinism rot risk. Also worse cache behavior (`ResolvedTerrainCell` ~120 bytes vs `PathCell` ~8).
- **Approach C — encode diff-1-slope-0 edges as cost=0 in `TerrainCostGrid`.** Rejected: cost grid is per-cell, not per-edge; cannot distinguish entry direction. Ramps would either over-block (both directions) or under-block (no parity gain).
- **Narrower scope (G5 diff-1 only).** Rejected per parity-bar rule: diff≥2 walkthrough is observable on the same retail terrain (every stepped cliff face), so cutting it leaves a known player-visible gap with no implementation savings (one extra match arm).
- **Wider scope (full Can_Enter_Cell port).** Deferred: the full predicate includes the two-pass mechanism (G6) and entity-block re-classification, which is a much larger surface area and has its own deferred-investigation status. This design closes the height-diff legality piece only.
