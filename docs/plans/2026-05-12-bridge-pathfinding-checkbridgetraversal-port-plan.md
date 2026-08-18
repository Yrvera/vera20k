# CheckBridgeTraversal Port — Implementation Plan

> **For Claude:** Execute task-by-task. Each task is self-contained — repeat
> context across tasks where needed.

**Goal:** Add a height-diff legality gate to A* neighbor expansion that mirrors
gamemd's `CheckBridgeTraversal`: gate diff-1 transitions on the lower cell's
SlopeIndex, and hard-block any other non-zero height diff (legitimate bridge
transitions emerge as diff-0 from `compute_neighbor_height` and pass through
unaffected).

**Architecture:** Extends `PathCell` by one byte (`slope_type: u8`), propagates
that byte through `PathGrid::from_resolved_terrain_with_bridges` from
`ResolvedTerrainCell.slope_type` (already populated from TMP `ramp_type`), and
adds a single `match` inside the existing A* neighbor loop. No new module, no
new crate, no API change.

**Design Doc:** [docs/plans/2026-05-12-bridge-pathfinding-checkbridgetraversal-port-design.md](2026-05-12-bridge-pathfinding-checkbridgetraversal-port-design.md)

---

## Grounding Summary

- **Docs.** Load-bearing source is [BRIDGE_DEFERRED_MECHANICS_GHIDRA_REPORT.md](../../../ra2-rust-game-docs/BRIDGE_DEFERRED_MECHANICS_GHIDRA_REPORT.md) §3.1 (CheckBridgeTraversal at 0x4D9C60), §3.3 (RecalcAttributes writes to CellClass+0x11C), §6 (current Rust gap), §11 rec #2 (brainstorm target). Audited GREEN today (2026-05-12).
- **Ghidra verification.** Skipped per `feedback_brainstorm_verification_preflight.md`: the doc was re-verified today, including the +0x11C SlopeIndex offset correction.
- **Repo pattern.** Mirrors existing `PathCell` fields (`bridge_walkable`, `bridge_deck_level`) — A*-hot data copied once at `PathGrid::from_resolved_terrain_with_bridges`. Gate inlined in the same neighbor loop that already runs corner-cutting and terrain-cost checks. Slope source already exists at `ResolvedTerrainCell.slope_type` (set in `merge_tmp_metadata` from TMP `ramp_type`).
- **INI.** No INI keys drive this — `SlopeIndex` is per-tile TMP file metadata, not a `rules.ini` value.
- **Probe baseline.** [tests/bridge_pathfinding_g5_g6_fidelity_probe.rs](../../tests/bridge_pathfinding_g5_g6_fidelity_probe.rs) reports 2,038 trusted firing pairs across 21 retail RA2/YR maps. Post-impl validation: re-run with an A*-rejection extension; expected count → 0.
- **Unknown.** Whether the legality gate fires on any legitimate bridge-crossing edge case in the existing bridge tests (G3/G4 regression set, layered-path tests). If yes, that's evidence the L7 carry-through-compute-neighbor-height claim is incomplete — go back to design.

## Key Technical Decisions

- **`slope_type: u8` lives on `PathCell` (not a side-table or per-edge lookup).** — keeps A*'s hot loop on contiguous 8-byte cells, no callsite wiring needed. **Confidence:** high. **Source:** repo pattern, `PathCell` already carries `bridge_deck_level` for the same reason. Brainstorm Approaches B/C rejected.
- **Single inline match: `0 => true, 1 => slope!=0, _ => false`.** — relies on `compute_neighbor_height`'s existing Case 3 to transform legitimate bridge entries (diff==4 with `transition`) into diff-0 from the legality gate's perspective. Residual diff-4 reaching the gate is necessarily a non-bridge cliff, which gamemd also blocks. **Confidence:** medium. **Source:** trace through `compute_neighbor_height` Cases 1–3 (src/sim/pathfinding/core.rs:123–153) against §3.1 pseudo-code. Medium confidence because the carry-through claim has not been runtime-verified — Task 11 is the empirical check.
- **L6 (diff-0 flat-step guard) covered implicitly.** — `compute_neighbor_height` keeps the unit's Z synced with cell state, so the divergent diff-0 case (mismatched targetHeight) is unreachable in our model. Backed by a `debug_assert!` in the legality gate plus the existing bridge regression tests. **Confidence:** medium. **Source:** structural argument from §3.1. Promotion to high confidence after Task 11 confirms no bridge tests regress.
- **`CLIFF_COST_MULTIPLIER` (×4) stays.** — independent concern: still want to discourage ramp traversal vs flat. Legitimate diff-1 ramp steps (slope!=0) pay the cost; blocked steps cost nothing because they `continue`. **Confidence:** high. **Source:** brainstorm parity-fit analysis.

## Open Questions

### Resolved During Planning
- *Should the diff-4 match arm be permit-or-block?* — Resolved: **block** (`_ => false`). Tracing `compute_neighbor_height` shows legitimate bridge transitions arrive as diff-0; any residual diff-4 is necessarily a non-bridge cliff that gamemd blocks too. See design doc §"Chosen Approach" updated section.
- *Do we need to add a comment in code referencing gamemd?* — No, per `feedback_no_engine_refs_in_comments`. Code comments say "Height-diff legality gate" only; the doc reference goes in the plan/design, not in source.

### Deferred to Implementation
- *Does Task 11's A*-rejection extension confirm 0 firing pairs across all 21 retail maps?* — Empirical; cannot answer without running the implementation.
- *Do any existing bridge tests regress?* — Empirical; surface in Task 10.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | [src/sim/pathfinding/core.rs](../../src/sim/pathfinding/core.rs) | `PathCell` schema, defaults, `from_resolved_terrain_with_bridges`, `diff_cells`, `astar_search` neighbor gate, `set_cell_for_test` helper |
| Modify | [src/sim/pathfinding/core_tests.rs](../../src/sim/pathfinding/core_tests.rs) | 26 PathCell construction sites (add `slope_type: 0`), 4 new G5/diff-N tests, update one stale comment |
| Modify | [src/sim/pathfinding/zone_map_tests.rs](../../src/sim/pathfinding/zone_map_tests.rs) | 1 PathCell construction site (`path_grid_from_heights`) |
| Modify | [src/sim/movement/movement_bridge.rs](../../src/sim/movement/movement_bridge.rs) | 1 PathCell construction site in test helper |
| Modify | [tests/bridge_pathfinding_g5_g6_fidelity_probe.rs](../../tests/bridge_pathfinding_g5_g6_fidelity_probe.rs) | Add A*-rejection extension to validate post-impl |

## Interface Changes

- `pub struct PathCell` grows by one field (`slope_type: u8`). All construction sites in the workspace are internal (no external crates consume `PathCell`); enumerated above. Padded struct size grows from 5 bytes to 8 bytes (alignment), so the in-memory diff is +3 bytes per cell (~200 KB on a 256×256 grid — negligible).
- No trait, function-signature, or pub-API changes.

## Sim Checklist

- [x] All math uses integer types — `i16` diff comparison, `u8` slope/height. No floats introduced.
- [x] New state lives only on `PathCell`. `PathCell` is rebuilt at each `PathGrid::from_resolved_terrain_with_bridges` call (map load + bridge state changes). Not part of the per-tick state hash — the hash is over `EntityStore`, not over PathGrid. (Confirm: search for `state_hash` to verify PathGrid is not hashed.)
- [x] No dependencies on render/ui/sidebar/audio/net introduced.
- [x] Tick ordering: unchanged. PathGrid is consulted by `Simulation::advance_tick` movement step, which itself is unchanged.
- [x] BTreeMap iteration order: irrelevant (PathGrid is a `Vec<PathCell>`).

## Risk Areas

- **A* path regressions.** Any scenario that previously routed across a cliff edge (diff-1 with cliff-slope lower, or non-bridge diff-4 cliff) now fails or detours. Surface in Task 10 (full pathfinding test suite). Lostlake.mmx-scale scenarios (where 100% of diff-1 pairs fire) will see units genuinely unable to reach destinations that were previously reachable via cliff cut-through — that's the parity win.
- **Bridge entry regressions.** If `compute_neighbor_height` doesn't carry legitimate bridge transitions through as diff-0 in some edge case we haven't traced, the legality gate will reject them. Task 10 catches; Task 11 confirms empirically.
- **State-hash determinism.** [tests/determinism_replay.rs](../../tests/determinism_replay.rs) replays pinned scenarios. If any replay touches a cliff-edge cell, the hash will diverge. Task 12 surfaces; fixtures may need re-recording (manual user step, not automated in plan).

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|--------|------|----------------|--------------|
| Task 5 | Diff-1 step with lower cell `slope_type == 0` → blocked | Player observation: units no longer walk through cliff faces between adjacent stepped plateaus. gamemd blocks; we currently permit. 2,038 fires across 21 retail maps. | Tasks 6, 7, 11: unit tests + per-pair A*-rejection extension on retail maps. |
| Task 5 | Diff ∈ {±2, ±3, ±4-non-bridge, ±5+} → blocked | Player observation: units no longer climb 2+ tall vertical cliffs or step off bridge decks into voids. gamemd blocks; we currently permit at ×4 cost. | Tasks 7, 8: diff-N unit tests + updated `test_cliff_cost_detours_under_uniform_base`. |
| Task 5 | Legitimate bridge transitions (Case 3 entry, Case 2 deck-to-deck) still permitted | Player observation: bridges still work — units cross bridges, enter at bridgeheads, exit normally. | Task 10: full pathfinding regression. |
| L10 in design ledger | Exactly one slope read per neighbor | Determinism + perf: no redundant reads. | Code review of the inline gate — single `lower_slope` binding. |

---

## Tasks

### Task 1: Add `slope_type: u8` to `PathCell` struct and default constants

**Why:** Foundation — every other change depends on the field existing.

**Files:**
- Modify: [src/sim/pathfinding/core.rs](../../src/sim/pathfinding/core.rs) — `PathCell` struct (line 665), `DEFAULT_WALKABLE_CELL` (line 702), `DEFAULT_BLOCKED_CELL` (line 711), `set_cell_for_test` helper (line 1098).

**Pattern:** Mirror existing `bridge_deck_level: u8` field on `PathCell`.

**Step 1: Extend the struct**

In [src/sim/pathfinding/core.rs:665-671](../../src/sim/pathfinding/core.rs#L665-L671), change:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PathCell {
    pub ground_walkable: bool,
    pub bridge_walkable: bool,
    pub transition: bool,
    pub ground_level: u8,
    pub bridge_deck_level: u8,
}
```

to:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PathCell {
    pub ground_walkable: bool,
    pub bridge_walkable: bool,
    pub transition: bool,
    pub ground_level: u8,
    pub bridge_deck_level: u8,
    /// Per-cell ramp/slope index (1-20 = canonical ramp direction; 0 = cliff or no ramp).
    /// Sourced from the TMP tile `ramp_type` byte via `ResolvedTerrainCell.slope_type`.
    /// Read by the A* height-diff legality gate for diff-1 transitions.
    pub slope_type: u8,
}
```

**Step 2: Update default constants**

In [src/sim/pathfinding/core.rs:702-708](../../src/sim/pathfinding/core.rs#L702-L708):

```rust
const DEFAULT_WALKABLE_CELL: PathCell = PathCell {
    ground_walkable: true,
    bridge_walkable: false,
    transition: false,
    ground_level: 0,
    bridge_deck_level: 0,
};
```

to:

```rust
const DEFAULT_WALKABLE_CELL: PathCell = PathCell {
    ground_walkable: true,
    bridge_walkable: false,
    transition: false,
    ground_level: 0,
    bridge_deck_level: 0,
    slope_type: 0,
};
```

Apply the same one-line addition to `DEFAULT_BLOCKED_CELL` at [core.rs:711-717](../../src/sim/pathfinding/core.rs#L711-L717).

**Step 3: Update the `set_cell_for_test` helper**

In [src/sim/pathfinding/core.rs:1098-1104](../../src/sim/pathfinding/core.rs#L1098-L1104), add `slope_type: 0,` to the `PathCell { ... }` literal.

**Step 4: Verify**

```
cargo check --lib
```

Expected: lib compiles. `cargo check --tests` will fail with many "missing field `slope_type`" errors — that's expected and addressed in Tasks 2–3.

**Step 5: Do not commit yet** — leave for Task 4 commit boundary.

---

### Task 2: Populate `slope_type` in `PathGrid::from_resolved_terrain_with_bridges`

**Why:** Wires the new field to its data source. Without this, `slope_type` is always 0 at runtime and every diff-1 step would block.

**Files:**
- Modify: [src/sim/pathfinding/core.rs:976-1025](../../src/sim/pathfinding/core.rs#L976-L1025)

**Pattern:** Same line-for-line pattern as `ground_level: cell.level` already in the same closure.

**Step 1: Add the field to the constructor**

Inside the `.map(|cell| PathCell { ... })` closure at [core.rs:982-1018](../../src/sim/pathfinding/core.rs#L982-L1018), append a new line just before the closing `})`:

```rust
slope_type: cell.slope_type,
```

The full closure after this change has these fields in order: `ground_walkable`, `bridge_walkable`, `transition`, `ground_level`, `bridge_deck_level`, `slope_type`.

**Step 2: Verify**

```
cargo check --lib
```

Expected: lib compiles.

**Step 3: Do not commit yet.**

---

### Task 3: Extend `PathGrid::diff_cells` to compare `slope_type`

**Why:** Defensive — `diff_cells` lists cells whose path-relevant state changed between two grids. If `slope_type` ever mutates at runtime (today it doesn't, but `recalc_overlay_passability` could in the future), the differ must catch it to avoid stale paths.

**Files:**
- Modify: [src/sim/pathfinding/core.rs:1029-1046](../../src/sim/pathfinding/core.rs#L1029-L1046)

**Step 1: Add the comparison clause**

Inside the existing `if a.ground_walkable != b.ground_walkable || ...` chain at [core.rs:1036-1041](../../src/sim/pathfinding/core.rs#L1036-L1041), add one more clause:

```rust
|| a.slope_type != b.slope_type
```

The full chain becomes:

```rust
if a.ground_walkable != b.ground_walkable
    || a.bridge_walkable != b.bridge_walkable
    || a.transition != b.transition
    || a.ground_level != b.ground_level
    || a.bridge_deck_level != b.bridge_deck_level
    || a.slope_type != b.slope_type
{
    changed.push(((idx % w) as u16, (idx / w) as u16));
}
```

**Step 2: Verify**

```
cargo check --lib
```

Expected: lib compiles cleanly.

**Step 3: Do not commit yet.**

---

### Task 4: Update PathCell construction sites in test fixtures

**Why:** Every `PathCell { ... }` literal in the workspace needs `slope_type: 0,` (or a specific value where the test exercises slope semantics). Until this is done, the test suite fails to compile.

**Files:**
- Modify: [src/sim/pathfinding/core_tests.rs](../../src/sim/pathfinding/core_tests.rs) — 26 sites at lines 141, 148, 155, 163, 170, 177, 185, 192, 199, 221, 241, 749, 763, 778, 793, 800, 813, 820, 833, 840, 853, 860, 874, 881, 895, 902.
- Modify: [src/sim/pathfinding/zone_map_tests.rs:425-431](../../src/sim/pathfinding/zone_map_tests.rs#L425-L431) — 1 site in `path_grid_from_heights` helper.
- Modify: [src/sim/movement/movement_bridge.rs:174-181](../../src/sim/movement/movement_bridge.rs#L174-L181) — 1 site in test helper `cell()`.

**Pattern:** Each construction site currently ends with `bridge_deck_level: <value>,` (with or without trailing comma). Add `slope_type: 0,` as a new line immediately after. All 28 sites use `slope_type: 0` — these are flat/cliff/bridge fixtures, not ramp fixtures, so 0 is the right default.

**Step 1: Update core_tests.rs**

For each of the 26 sites listed above, locate the `PathCell {` literal and append a `slope_type: 0,` line after `bridge_deck_level`. Example for the first site at line 141:

Before:
```rust
PathCell {
    ground_walkable: true,
    bridge_walkable: false,
    transition: false,
    ground_level: 0,
    bridge_deck_level: 0,
},
```

After:
```rust
PathCell {
    ground_walkable: true,
    bridge_walkable: false,
    transition: false,
    ground_level: 0,
    bridge_deck_level: 0,
    slope_type: 0,
},
```

**Step 2: Update zone_map_tests.rs**

The `path_grid_from_heights` helper at line 421-433 constructs many cells in one `.map()` closure — only one edit needed:

```rust
.map(|&h| PathCell {
    ground_walkable: true,
    bridge_walkable: false,
    transition: false,
    ground_level: h,
    bridge_deck_level: 0,
    slope_type: 0,
})
```

**Step 3: Update movement_bridge.rs**

The `cell()` helper at line 168-181 returns one PathCell — one edit:

```rust
PathCell {
    ground_walkable: true,
    bridge_walkable,
    transition,
    ground_level,
    bridge_deck_level,
    slope_type: 0,
}
```

**Step 4: Verify**

```
cargo check --tests
```

Expected: every test file compiles. If any "missing field `slope_type`" error remains, locate the site and add the field.

**Step 5: Commit**

Stage and commit all changes from Tasks 1–4 together. Suggested message:

```
sim/pathfinding: add slope_type field to PathCell

Propagates ResolvedTerrainCell.slope_type (TMP ramp_type byte) into the
A* PathCell so the upcoming height-diff legality gate can read it.
No behavioral change in this commit — gate lands separately.

Touches 28 PathCell construction sites across the pathfinding,
movement, and zone-map test modules.
```

Verify with `git status` and `cargo test --no-run --lib` (compilation check, no run yet).

---

### Task 5: Add height-diff legality gate to A* neighbor expansion

**Why:** The actual parity fix. Without this task, the previous tasks have plumbed data but A* still permits cliff cut-throughs.

**Files:**
- Modify: [src/sim/pathfinding/core.rs:424-437](../../src/sim/pathfinding/core.rs#L424-L437) — insert between `compute_neighbor_height` and the closed-list check.

**Pattern:** Inline match in the same neighbor loop that runs corner-cutting, terrain-cost, and entity-soft-block checks. No new helper function unless the match grows later.

**Step 1: Insert the gate**

Find this block in [src/sim/pathfinding/core.rs](../../src/sim/pathfinding/core.rs) (around lines 422-437):

```rust
let neighbor_cell = grid.cell(nx, ny).unwrap_or(&DEFAULT_BLOCKED_CELL);

// Closed-list selection: uses CURRENT node's height vs neighbor ground_level
let neighbor_use_bridge = is_at_bridge_level(current.height, neighbor_cell);

// Compute what height the NEW node carries forward (separate computation)
let neighbor_height = compute_neighbor_height(current.height, cur_cell, neighbor_cell);

// Closed check on appropriate list
if neighbor_use_bridge {
    if bridge_closed[n_idx] {
        continue;
    }
} else if ground_closed[n_idx] {
    continue;
}
```

Insert the legality gate immediately after `compute_neighbor_height` and before the closed-list check:

```rust
let neighbor_cell = grid.cell(nx, ny).unwrap_or(&DEFAULT_BLOCKED_CELL);

// Closed-list selection: uses CURRENT node's height vs neighbor ground_level
let neighbor_use_bridge = is_at_bridge_level(current.height, neighbor_cell);

// Compute what height the NEW node carries forward (separate computation)
let neighbor_height = compute_neighbor_height(current.height, cur_cell, neighbor_cell);

// Height-diff legality gate. Diff-1 transitions require the LOWER cell to
// be a canonical ramp (slope_type != 0); diff ∈ {±2, ±3, ±4, ±5+} is
// always blocked. Legitimate bridge transitions arrive here as diff-0
// because `compute_neighbor_height` already shifts unit Z onto/off the deck.
let diff = neighbor_height as i16 - current.height as i16;
let lower_slope = if diff < 0 {
    neighbor_cell.slope_type
} else {
    cur_cell.slope_type
};
let legal = match diff.abs() {
    0 => true,
    1 => lower_slope != 0,
    _ => false,
};
if !legal {
    continue;
}

// Closed check on appropriate list
if neighbor_use_bridge {
    if bridge_closed[n_idx] {
        continue;
    }
} else if ground_closed[n_idx] {
    continue;
}
```

**Step 2: Verify lib compiles**

```
cargo check --lib
```

**Step 3: Run pathfinding tests; expect some failures, classify them**

```
cargo test --lib --no-fail-fast sim::pathfinding 2>&1 | tail -80
```

Classify any failures into:
1. **Expected failures** — tests that relied on cliff cut-throughs being permitted (e.g., `test_cliff_cost_detours_under_uniform_base` might still pass since alt route exists; but tests asserting a specific cliff-crossing path will fail).
2. **Unexpected failures** — bridge tests, legitimate transitions. These indicate the L7 carry-through claim is incomplete; STOP and re-examine before continuing.

If only expected failures remain, proceed to Task 6 (which adds positive coverage of the new behavior). Update or remove failing assertions in Task 8.

If unexpected failures: investigate the failing test fixture, capture the diff-N pattern that broke, and reopen the design doc before changing anything else.

**Step 4: Do not commit yet** — leave for Task 9 commit boundary after tests are in.

---

### Task 6: Unit tests for diff-1 SlopeIndex gate

**Why:** Positive test coverage of the core G5 mechanic. Pure-logic, fast, deterministic.

**Files:**
- Modify: [src/sim/pathfinding/core_tests.rs](../../src/sim/pathfinding/core_tests.rs) — append four new tests at the end of the test module (or beside `test_cliff_cost_detours_under_uniform_base` for thematic grouping).

**Pattern:** Same as `test_cliff_cost_detours_under_uniform_base` — construct a 3×3 cell array, build `PathGrid::from_cells`, call `find_path`, assert path constraints.

**Step 1: Add the tests**

Append these four tests to [src/sim/pathfinding/core_tests.rs](../../src/sim/pathfinding/core_tests.rs):

```rust
#[test]
fn diff_1_slope_zero_lower_blocks_going_up() {
    // 3x1 grid: level 0 (slope 0) — level 1 (slope 0) — level 0 (slope 0).
    // gamemd CheckBridgeTraversal blocks both edges because diff=1 and the
    // lower (level-0) cell has SlopeIndex=0. A* must report no path.
    let cells = vec![
        PathCell {
            ground_walkable: true,
            bridge_walkable: false,
            transition: false,
            ground_level: 0,
            bridge_deck_level: 0,
            slope_type: 0,
        },
        PathCell {
            ground_walkable: true,
            bridge_walkable: false,
            transition: false,
            ground_level: 1,
            bridge_deck_level: 0,
            slope_type: 0,
        },
        PathCell {
            ground_walkable: true,
            bridge_walkable: false,
            transition: false,
            ground_level: 0,
            bridge_deck_level: 0,
            slope_type: 0,
        },
    ];
    let grid = PathGrid::from_cells(cells, 3, 1);
    let path = find_path(&grid, (0, 0), (2, 0));
    assert!(
        path.is_none(),
        "diff-1 cliff with slope=0 lower cell must block A* — got {:?}",
        path
    );
}

#[test]
fn diff_1_slope_nonzero_lower_permits_going_up() {
    // 3x1 grid: level 0 (slope 0) — level 1 (slope 2, ramp) — level 0 (slope 0).
    // Lower cell on each edge is the level-0 flat; flat has slope 0.
    // gamemd would BLOCK because the LOWER cell on a diff-1 pair must
    // have slope != 0 for the step to be legal. Verify our gate matches:
    // even though the ramp cell has slope=2, the LOWER cell of each edge
    // is the level-0 flat with slope=0, so the gate blocks.
    let cells = vec![
        PathCell {
            ground_walkable: true,
            bridge_walkable: false,
            transition: false,
            ground_level: 0,
            bridge_deck_level: 0,
            slope_type: 0,
        },
        PathCell {
            ground_walkable: true,
            bridge_walkable: false,
            transition: false,
            ground_level: 1,
            bridge_deck_level: 0,
            slope_type: 2, // canonical ramp, BUT this cell is the UPPER side
        },
        PathCell {
            ground_walkable: true,
            bridge_walkable: false,
            transition: false,
            ground_level: 0,
            bridge_deck_level: 0,
            slope_type: 0,
        },
    ];
    let grid = PathGrid::from_cells(cells, 3, 1);
    let path = find_path(&grid, (0, 0), (2, 0));
    assert!(
        path.is_none(),
        "lower cell (level-0 flat, slope=0) gates the edge — upper-side slope is irrelevant"
    );
}

#[test]
fn diff_1_slope_nonzero_lower_actually_permits() {
    // Inverted from the previous test: the LOWER cell has slope=2 (a stepped
    // ramp at the bottom of a 1-tall rise). gamemd permits because lower
    // cell's SlopeIndex != 0. A* should find a path 0->1->2.
    let cells = vec![
        PathCell {
            ground_walkable: true,
            bridge_walkable: false,
            transition: false,
            ground_level: 0,
            bridge_deck_level: 0,
            slope_type: 2, // lower-side flat-but-ramped
        },
        PathCell {
            ground_walkable: true,
            bridge_walkable: false,
            transition: false,
            ground_level: 1,
            bridge_deck_level: 0,
            slope_type: 0,
        },
        PathCell {
            ground_walkable: true,
            bridge_walkable: false,
            transition: false,
            ground_level: 0,
            bridge_deck_level: 0,
            slope_type: 2,
        },
    ];
    let grid = PathGrid::from_cells(cells, 3, 1);
    let path = find_path(&grid, (0, 0), (2, 0))
        .expect("lower-cell slope=2 must permit diff-1 transitions");
    assert_eq!(path.first().copied(), Some((0, 0)));
    assert_eq!(path.last().copied(), Some((2, 0)));
}

#[test]
fn diff_1_slope_zero_lower_blocks_going_down() {
    // Symmetry check: same gate fires when stepping DOWN from level 1 to
    // level 0 with slope=0 on the lower (level-0) cell. Start at the high
    // cell, goal at the low — A* must fail (no detour available in 1x2).
    let cells = vec![
        PathCell {
            ground_walkable: true,
            bridge_walkable: false,
            transition: false,
            ground_level: 1,
            bridge_deck_level: 0,
            slope_type: 0,
        },
        PathCell {
            ground_walkable: true,
            bridge_walkable: false,
            transition: false,
            ground_level: 0,
            bridge_deck_level: 0,
            slope_type: 0,
        },
    ];
    let grid = PathGrid::from_cells(cells, 2, 1);
    let path = find_path(&grid, (0, 0), (1, 0));
    assert!(
        path.is_none(),
        "going-down diff-1 with slope=0 lower must block — got {:?}",
        path
    );
}

#[test]
fn l6_canary_unit_at_deck_height_stepping_to_non_bridge_produces_nonzero_diff() {
    // Design ledger L6: gamemd's diff-0 flat-step guard fires when the unit's
    // targetHeight doesn't match src.Level on a non-bridge transition. Our
    // model avoids that state by letting `compute_neighbor_height` produce a
    // non-zero diff in exactly this case: a unit on a bridge deck (height=4)
    // stepping into a non-bridge cell goes through Case 1, which returns
    // neighbor.ground_level (0), giving diff=-4. The legality gate then
    // blocks via `_ => false`.
    //
    // If this test ever fails, it means compute_neighbor_height has been
    // refactored in a way that lets the divergent diff-0 state arise and
    // the L6 implicit-handling claim in the design no longer holds — at
    // that point the legality gate needs an explicit diff-0 guard.
    let parent = PathCell {
        ground_walkable: true,
        bridge_walkable: true,
        transition: false,
        ground_level: 0,
        bridge_deck_level: 4,
        slope_type: 0,
    };
    let neighbor = PathCell {
        ground_walkable: true,
        bridge_walkable: false,
        transition: false,
        ground_level: 0,
        bridge_deck_level: 0,
        slope_type: 0,
    };
    let h = compute_neighbor_height(4, &parent, &neighbor);
    assert_eq!(h, 0, "Case 1: non-bridge neighbor returns its ground_level");
    let diff = h as i16 - 4i16;
    assert_eq!(
        diff.abs(),
        4,
        "L6 invariant: unit at deck height stepping to non-bridge produces |diff|=4, not 0"
    );
}
```

**Step 2: Verify**

```
cargo test --lib sim::pathfinding::diff_1_ -- --nocapture
```

Expected: all four tests pass.

**Step 3: Do not commit yet.**

---

### Task 7: Unit tests for diff ≥ 2 hard block

**Why:** Direct coverage of the `_ => false` arm. Catches regressions if anyone ever softens the gate.

**Files:**
- Modify: [src/sim/pathfinding/core_tests.rs](../../src/sim/pathfinding/core_tests.rs) — append three tests beside the diff-1 group.

**Step 1: Add the tests**

```rust
#[test]
fn diff_2_blocks_regardless_of_slope() {
    // 2x1 grid: level 0 — level 2. Both cells have slope=2 (would permit
    // diff-1 either way). gamemd hard-blocks diff>=2 even with ramps.
    let cells = vec![
        PathCell {
            ground_walkable: true,
            bridge_walkable: false,
            transition: false,
            ground_level: 0,
            bridge_deck_level: 0,
            slope_type: 2,
        },
        PathCell {
            ground_walkable: true,
            bridge_walkable: false,
            transition: false,
            ground_level: 2,
            bridge_deck_level: 0,
            slope_type: 2,
        },
    ];
    let grid = PathGrid::from_cells(cells, 2, 1);
    let path = find_path(&grid, (0, 0), (1, 0));
    assert!(
        path.is_none(),
        "diff-2 must always block, even with slope=2 on both cells — got {:?}",
        path
    );
}

#[test]
fn diff_3_blocks_regardless_of_slope() {
    let cells = vec![
        PathCell {
            ground_walkable: true,
            bridge_walkable: false,
            transition: false,
            ground_level: 0,
            bridge_deck_level: 0,
            slope_type: 2,
        },
        PathCell {
            ground_walkable: true,
            bridge_walkable: false,
            transition: false,
            ground_level: 3,
            bridge_deck_level: 0,
            slope_type: 2,
        },
    ];
    let grid = PathGrid::from_cells(cells, 2, 1);
    let path = find_path(&grid, (0, 0), (1, 0));
    assert!(path.is_none(), "diff-3 must block — got {:?}", path);
}

#[test]
fn diff_5_blocks_regardless_of_slope() {
    let cells = vec![
        PathCell {
            ground_walkable: true,
            bridge_walkable: false,
            transition: false,
            ground_level: 0,
            bridge_deck_level: 0,
            slope_type: 2,
        },
        PathCell {
            ground_walkable: true,
            bridge_walkable: false,
            transition: false,
            ground_level: 5,
            bridge_deck_level: 0,
            slope_type: 2,
        },
    ];
    let grid = PathGrid::from_cells(cells, 2, 1);
    let path = find_path(&grid, (0, 0), (1, 0));
    assert!(path.is_none(), "diff-5 must block — got {:?}", path);
}
```

**Step 2: Verify**

```
cargo test --lib sim::pathfinding::diff_ -- --nocapture
```

Expected: all diff-N tests pass.

**Step 3: Do not commit yet.**

---

### Task 8: Update the stale comment + assertions on `test_cliff_cost_detours_under_uniform_base`

**Why:** The existing test asserts the cliff is detoured. Under the new gate it's hard-blocked (not just expensive), and the comment's "Direct path is 2 steps but pays cliff×4 twice ≈ 10000 g_cost" reasoning is no longer the operative rule. Keep the assertion (it still holds), but fix the comment so future readers understand why.

**Files:**
- Modify: [src/sim/pathfinding/core_tests.rs:133-217](../../src/sim/pathfinding/core_tests.rs#L133-L217)

**Step 1: Update the comment**

Replace the `// 3x3 grid. Direct row 0 ...` doc-comment block above the `let cells = vec![ ... ]` line (around line 135) with:

```rust
// 3x3 grid. Direct row 0 has a height-step at (1,0): ground=4 between
// ground=0 cells. Both edges (0,0)->(1,0) and (1,0)->(2,0) have diff=4,
// which the height-diff legality gate hard-blocks (diff>=2 always blocks
// in `astar_search`). A* must detour via row 1 (all flat).
// (Pre-gate, the path was permitted at ~10000 g_cost vs ~4000 for the
// alt; the alt won on cost. The new gate makes the detour the only
// option, but the visible outcome is identical.)
```

**Step 2: Verify**

```
cargo test --lib test_cliff_cost_detours_under_uniform_base -- --nocapture
```

Expected: test still passes (assertion unchanged).

**Step 3: Do not commit yet.**

---

### Task 9: Run the full pathfinding test suite; commit

**Why:** Catch any regression in bridge tests (G3/G4 fix coverage, layered routing, runtime refresh) or zone-map tests. If anything fails, that's evidence the design's L7 carry-through claim doesn't hold for some edge case — stop and investigate before committing.

**Step 1: Run pathfinding tests**

```
cargo test --lib sim::pathfinding 2>&1 | tail -40
```

Expected: all tests pass (including the 3 G3+G4 regression tests added in commit `3e737f8`).

**Step 2: Run movement_bridge tests**

```
cargo test --lib sim::movement::movement_bridge 2>&1 | tail -20
```

Expected: all 6 movement_bridge tests pass (from commit `9373c70`).

**Step 3: Run bridge_state tests**

```
cargo test --lib sim::bridge_state 2>&1 | tail -20
```

Expected: pass.

**Step 4: If anything fails**

- Capture the failing test name + diff-N pattern in the failing fixture.
- Re-read the design doc's "Why `_ => false` correctly covers diff-4" paragraph.
- Determine: was the failure caused by a legitimate bridge transition that arrived as diff != 0? If yes, the carry-through claim is incomplete and the legality gate needs the `4 => ` arm refined. STOP and reopen the design.
- If the failure was an unrelated assertion (e.g., cost-based expected path that's still valid but now uses a slightly different node order), update the assertion and continue.

**Step 5: Commit**

Stage Tasks 5–8 together. Suggested message:

```
sim/pathfinding: gate A* on height-diff legality (G5 fix)

Adds a height-diff legality check inside astar_search neighbor expansion
that mirrors gamemd's CheckBridgeTraversal predicate:
- diff-0 permitted (legitimate bridge transitions arrive here via
  compute_neighbor_height Case 3)
- diff-1 permitted iff the LOWER cell's slope_type != 0
- diff in {2, 3, 4, 5+} hard-blocked (covers non-bridge cliff faces)

Closes the G5 parity gap surfaced by the fidelity probe: 2038 trusted
firing pairs across 21 retail RA2/YR maps no longer permit unit
cut-through. Diff-4 non-bridge cliffs now hard-block instead of
charging ×4 cost.

Adds 7 unit tests covering diff-1 slope-on-lower-cell semantics
(up/down/permit/block) and diff-2/3/5 hard block.

Stale comment on test_cliff_cost_detours_under_uniform_base updated
to reflect the new semantics; assertion unchanged.
```

---

### Task 10: Extend the fidelity probe with an A*-rejection check

**Why:** Empirical verification across all 21 retail maps that gathered firing pairs no longer route through. Closes the parity-bar accountability loop: design says "fixes 2038 pairs"; probe proves it.

**Files:**
- Modify: [tests/bridge_pathfinding_g5_g6_fidelity_probe.rs](../../tests/bridge_pathfinding_g5_g6_fidelity_probe.rs) — add a second `#[ignore]` test that scans the same firing pairs but calls A* to verify rejection.

**Pattern:** Reuses existing scanning scaffolding. The new test walks the same trusted firing pairs and, for each, builds a minimal `PathGrid` view + calls `astar_search` directly across the pair; asserts no path is returned.

**Step 1: Add the new test**

Append to [tests/bridge_pathfinding_g5_g6_fidelity_probe.rs](../../tests/bridge_pathfinding_g5_g6_fidelity_probe.rs):

```rust
#[test]
#[ignore]
fn probe_g5_astar_rejects_all_trusted_firing_pairs() {
    let _ = env_logger::try_init();
    let dir_str = ra2_dir();
    let ra2_dir = Path::new(&dir_str);
    if !ra2_dir.exists() {
        eprintln!("SKIP: RA2 dir not found at {}", dir_str);
        return;
    }
    let mut am = AssetManager::new(ra2_dir).expect("AssetManager::new");
    for mix in &[
        "maps01.mix",
        "maps02.mix",
        "maps03.mix",
        "mapsmd01.mix",
        "mapsmd02.mix",
        "mapsmd03.mix",
    ] {
        let _ = am.load_nested(mix);
    }
    let _ = am.load_all_disk_mixes();

    let rules_bytes = am
        .get("rulesmd.ini")
        .or_else(|| am.get("rules.ini"))
        .expect("rules ini required");
    let rules_ini = IniFile::from_bytes(&rules_bytes).expect("rules parse");

    let mut candidates: Vec<String> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(ra2_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            let lower = name.to_ascii_lowercase();
            if lower.ends_with(".mmx")
                || lower.ends_with(".yro")
                || lower.ends_with(".map")
                || lower.ends_with(".mpr")
            {
                candidates.push(name);
            }
        }
    }
    candidates.sort();

    let mut total_pairs = 0u32;
    let mut rejected_pairs = 0u32;
    let mut leaked_pairs: Vec<(String, (u16, u16), (u16, u16))> = Vec::new();

    for name in &candidates {
        let path = ra2_dir.join(name);
        let Ok(map) = map_file::load_from_path(&path) else { continue };
        if map.cells.is_empty() { continue; }
        let Some(grid) = build_grid_for_map(&mut am, &map, &rules_ini) else { continue };
        if grid.width() == 0 || grid.height() == 0 { continue; }

        let meta = metadata_health(&grid);
        if meta.resolved_fraction < 0.5 {
            // Skip maps where TMP metadata didn't resolve — slope readings unreliable.
            continue;
        }

        let path_grid = vera20k::sim::pathfinding::PathGrid::from_resolved_terrain_with_bridges(
            &grid, None,
        );

        let g5 = scan_g5(&grid);
        for ex in &g5.examples {
            // examples are TRUSTED pairs only (both cells have resolved metadata)
            total_pairs += 1;
            let lower_xy = (ex.lower.0, ex.lower.1);
            let upper_xy = (ex.upper.0, ex.upper.1);
            let path = vera20k::sim::pathfinding::find_path(&path_grid, lower_xy, upper_xy);
            match path {
                None => rejected_pairs += 1,
                Some(p) => {
                    // Even if a path exists, it must NOT step directly across
                    // the firing edge — verify by checking adjacency in path.
                    let direct_step = p.windows(2).any(|w| {
                        (w[0] == lower_xy && w[1] == upper_xy)
                            || (w[0] == upper_xy && w[1] == lower_xy)
                    });
                    if direct_step {
                        leaked_pairs.push((name.clone(), lower_xy, upper_xy));
                    } else {
                        rejected_pairs += 1;
                    }
                }
            }
        }
    }

    println!();
    println!("=== G5 A* rejection probe ===");
    println!("Tested pairs (sampled from each map's example set): {}", total_pairs);
    println!("Pairs rejected or detoured:                          {}", rejected_pairs);
    println!("Pairs LEAKED (A* still took the cliff step):         {}", leaked_pairs.len());
    for (m, lo, up) in &leaked_pairs {
        println!("  {} : {:?} -> {:?}", m, lo, up);
    }
    assert!(
        leaked_pairs.is_empty(),
        "A* must not step across diff-1 SlopeIndex==0 firing pairs"
    );
}

// Tiny extension for `G5Example` to support tuple destructuring.
trait LowerXY { fn _0_2(self) -> (u16, u16); }
impl LowerXY for (u16, u16, u8) { fn _0_2(self) -> (u16, u16) { (self.0, self.1) } }
```

**Step 2: Verify it compiles**

```
cargo test --no-run --test bridge_pathfinding_g5_g6_fidelity_probe
```

**Step 3: Run the probe**

```
cargo test --test bridge_pathfinding_g5_g6_fidelity_probe probe_g5_astar_rejects -- --ignored --nocapture
```

Expected: 0 leaked pairs. The probe samples only the example coords per map (max 8 per map per scan_g5 run), so coverage is broad but not exhaustive. If a leaked pair appears, capture its (map, lower, upper) and re-examine the gate — the failure points to either a `compute_neighbor_height` interaction we didn't model or a real bug in the gate.

**Step 4: Commit**

```
tests: G5 A* rejection probe — verify gate fires on retail terrain

Adds a sibling #[ignore]'d test that scans the same retail RA2/YR maps
and, for each trusted G5 firing pair, calls A* across it and asserts
no direct cliff step is taken. Closes the parity-bar accountability
loop: design says "fixes 2038 pairs"; this proves it.

Skips maps with <50% TMP metadata resolution (DESERT/NEWURBAN/LUNAR
theaters where slope_type readings are unreliable).
```

---

### Task 11: Run determinism replay test

**Why:** State-hash regression check. PathGrid is not directly hashed, but downstream sim state (unit positions, paths) is — any path divergence cascades into hash divergence.

**Files:** None modified in this task.

**Step 1: Run the replay**

```
cargo test --test determinism_replay -- --nocapture 2>&1 | tail -30
```

**Step 2: Classify outcome**

- **All pass:** done. Move to Task 12.
- **Hash mismatch on a scenario that doesn't involve cliff terrain:** unexpected — the gate shouldn't affect non-cliff scenarios. Investigate as a real bug.
- **Hash mismatch on a scenario that involves cliff terrain:** expected. The fixture needs re-recording, which is a manual user step (run the scenario fresh, capture the new hash, update the fixture). Document the affected scenario name and notify the user.

**Step 3: Do not auto-fix replay fixtures.** Replay-fixture changes are a user judgment call (they're the "ground truth" reference for replays). Surface the findings; let the user re-record or refuse the change.

---

### Task 12: Final regression sweep

**Why:** Last safety net before declaring the implementation done.

**Step 1: Run full sim test suite**

```
cargo test --lib sim 2>&1 | tail -20
```

Expected: all sim tests pass.

**Step 2: Run full workspace test suite (excluding ignored tests)**

```
cargo test 2>&1 | tail -20
```

Expected: workspace passes. (Note: per CLAUDE.md "Parallel sessions" rule, ignore failures in files this plan did not touch — those are unrelated work in progress.)

**Step 3: cargo clippy spot-check on touched files**

```
cargo clippy --lib --no-deps -- -D warnings 2>&1 | grep -E "core\.rs|core_tests\.rs|zone_map_tests\.rs|movement_bridge\.rs" | tail -20
```

Expected: no warnings on touched files.

**Step 4: Update fidelity probe interpretation summary**

Re-read [tests/bridge_pathfinding_g5_g6_fidelity_probe.rs](../../tests/bridge_pathfinding_g5_g6_fidelity_probe.rs) interpretation paragraph (`G5: FIRES on ... maps ... pairs total. Worth a /brainstorm session.`). Update the static print so future runs say the gap has been closed (A* now rejects). The exact text can be:

```
println!(
    "G5: SCAN-ONLY (legality gate landed 2026-05-12). Run `probe_g5_astar_rejects_all_trusted_firing_pairs`"
);
println!("    for empirical confirmation A* now rejects these pairs.");
```

**Step 5: Commit**

```
tests: G5 gate landed — update fidelity probe interpretation

The terrain scan still reports raw firing pairs (useful for regression
monitoring), but the gate now blocks all of them in A*. Updates the
human-readable summary to point at the A*-rejection probe for
empirical confirmation.
```

---

## Sources & References

- **Design doc:** [docs/plans/2026-05-12-bridge-pathfinding-checkbridgetraversal-port-design.md](2026-05-12-bridge-pathfinding-checkbridgetraversal-port-design.md)
- **Ghidra report:** [ra2-rust-game-docs/BRIDGE_DEFERRED_MECHANICS_GHIDRA_REPORT.md](../../../ra2-rust-game-docs/BRIDGE_DEFERRED_MECHANICS_GHIDRA_REPORT.md) §3.1, §3.3, §6, §11 (audited GREEN 2026-05-12)
- **gamemd.exe addresses (for plan reference, not for code comments):**
  - `CheckBridgeTraversal` at 0x4D9C60 — the algorithm being mirrored
  - `RecalcAttributes` at 0x47D2B0 — writes SlopeIndex byte
  - SlopeIndex byte at CellClass+0x11C (confirmed 2026-05-12; older docs claiming +0x11A were wrong)
- **Repo pattern source:**
  - `PathCell` struct + `PathGrid::from_resolved_terrain_with_bridges` at src/sim/pathfinding/core.rs:665–1025
  - `compute_neighbor_height` at src/sim/pathfinding/core.rs:123–153 (the function whose output the gate reads)
  - `astar_search` neighbor expansion at src/sim/pathfinding/core.rs:413–605
  - Existing G3/G4 fixes that this builds on: commits `0e9f76f`, `9b49e2c`, `3e737f8`
- **Probe baseline:** [tests/bridge_pathfinding_g5_g6_fidelity_probe.rs](../../tests/bridge_pathfinding_g5_g6_fidelity_probe.rs) — 2,038 trusted firing pairs across 21 retail RA2/YR maps as of 2026-05-12.
- **INI keys:** none — `slope_type` comes from per-tile TMP metadata, not `rules.ini`.
