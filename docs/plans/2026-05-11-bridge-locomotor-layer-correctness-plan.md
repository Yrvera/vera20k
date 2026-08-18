# Bridge Locomotor Layer Correctness — Implementation Plan

> **For Claude:** Execute this plan task-by-task. Each task is self-contained.

**Goal:** Replace the reactive Z-based bridge-layer heuristic with gamemd.exe's exact cell-flag predicate, decouple `on_bridge` from `loco.layer`, delete the anticipatory `apply_bridge_lookahead_if_needed` workaround, and tighten A*'s Ground→Bridge gates (bridgehead flag + exact height-diff 4).

**Architecture:** Pure-function predicate helpers in `src/sim/movement/movement_bridge.rs` (mirrors the `bump_crush.rs` / `cell_entry.rs` style). A* changes localized to `compute_neighbor_height` Case 3 and the neighbor-walkability branch in `astar_search` (both in `src/sim/pathfinding/core.rs`). `on_bridge` and `BridgeOccupancy` become predicate-driven; `loco.layer` continues to follow A*'s `path_layers`.

**Design Doc:** [docs/plans/2026-05-11-bridge-locomotor-layer-correctness-design.md](2026-05-11-bridge-locomotor-layer-correctness-design.md)

---

## Grounding Summary

- **Verified RE (ra2-rust-game-docs/AUDIT_LOG.md 2026-05-11 entries):** on_bridge predicate exact form at gamemd 0x4B0F20 (entry asm 0x4B1812, exit asm 0x4B1830/0x4B184A), `g_BridgeZOffset_Drive = ftol(g_DriveHeightStep * 4)` at 0x4AF4A0 (binary proof bridge deck = 4 levels), bit 0x80 set at map-init time upstream of SetBridgeDirection (specific load path NOT traced — only the post-condition is verified), NESW/NWSE byte-identical (irrelevant — no Rust port of SetBridgeDirection).
- **Verified reference (BRIDGE_SYSTEM.md):** CheckBridgeTraversal at 0x4D9C60 — diff 0/1 per-case, diff 4 only for bridge entry, diff 2/3/5+ always blocked. A* at 0x429A90 uses dual closed lists; layer per step from `is_at_bridge_level`. Set_Destination 0x4AFD40 unconditional Z bump on dst.0x100.
- **Repo pattern mirrored:** pure-fn helpers in [src/sim/movement/bump_crush.rs](../../src/sim/movement/bump_crush.rs) and [src/sim/movement/cell_entry.rs](../../src/sim/pathfinding/cell_entry.rs) — small testable helpers with explicit I/O types, called by the cell-transition orchestrator.
- **Existing data:** `PathCell` already carries `ground_walkable`, `bridge_walkable` (= 0x100 flag analog), `transition` (= 0x200 bridgehead analog), `ground_level` (= raw +0x11B), `bridge_deck_level` (= effective deck Z). No new fields needed.
- **INI keys:** none. Bridge mechanics are runtime-engine behavior, not INI-driven.
- **Git state:** last touch on affected files was `ff404b5 style: cargo fmt across the tree` (pure formatting). The design's "current state" references all hold.
- **Existing test coverage:** no tests currently cover `resolve_cell_transition_bridge_state` or `apply_bridge_lookahead_if_needed` directly. `movement_tests.rs` has no `test.*bridge|test.*ramp|on_bridge_` matches. All new tests in this plan are net-adds.
- **Open after grounding:** Diff-1 SlopeIndex handling (out of scope — see design §Known Parity Boundary). True two-pass `Can_Enter_Cell` (out of scope — pre-decision via path_layers approximates the post-switch output).

## Key Technical Decisions

- **Predicate uses signed i8 arithmetic with `wrapping_sub`.** — **Confidence:** high. **Source:** AUDIT_LOG.md 2026-05-11 (MOVSX read at gamemd) + defensive against malformed maps with extreme height values.
- **Resolver returns only `BridgeStateUpdate`, not a layer.** — **Confidence:** high. **Source:** design doc §"Critical parity invariant". The layer continues to follow A*'s `path_layers`; the predicate drives `on_bridge` independently.
- **`apply_bridge_lookahead_if_needed` is deleted, not refactored.** — **Confidence:** high. **Source:** design doc §"Why our heuristic is wrong". The anticipatory layer change is a workaround for the broken primary mechanism; gamemd has no anticipatory layer change.
- **A* G3 fix gates Ground→Bridge crossings on `transition` flag in BOTH `compute_neighbor_height` AND the neighbor-walkability check.** — **Confidence:** high. **Source:** BRIDGE_SYSTEM.md §"CheckBridgeTraversal" + gap-scan G3.
- **A* G4 fix tightens Case 3 from `(2..=4)` to exactly 4.** — **Confidence:** high. **Source:** BRIDGE_SYSTEM.md §"CheckBridgeTraversal" + gap-scan G4.
- **`loco.layer` continues to track A*'s `path_layers` per step.** — **Confidence:** high. **Source:** matches gamemd's A* closed-list assignment; cell_entry/walkability read this for layer-aware occupancy lookup.

## Open Questions

### Resolved During Planning

- **What's the source of bit 0x80 on body cells in our `PathCell`?** Set at map-load time in our Rust port (already correctly populated by PathGrid construction). The exact gamemd load path that ultimately populates the binary's `cell+0x140 & 0x80` flag was not traced — only the post-condition is verified. No code change needed in the Rust port either way.
- **Does our `PathCell` need new fields for the predicate?** No — `ground_level` and `bridge_walkable` cover everything the predicate reads.
- **Where is `resolve_cell_transition_bridge_state` called?** Two sites: `movement_step.rs:664` (main cell-crossing path) and `movement_tick.rs:608` (drive_track cell-jump path). Both must be updated.
- **Where is `apply_pending_bridge_render_state` called?** Two sites: `movement_tick.rs:669` (drive_track-jump path) and `movement_tick.rs:786` (main tick path). Both must be updated for the new `BridgeStateUpdate` parameter type.
- **Where is `apply_bridge_lookahead_if_needed` called?** One site: `movement_tick.rs:806`. Delete the call along with the function.

### Deferred to Implementation

- **State hash determinism after the fix.** The fix changes when `on_bridge` flips relative to existing replays. Acceptable per design §"Impact Analysis" (no published replays). If a test relies on a specific tick when `on_bridge` flips, it may need timing adjustment.

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| Modify | [src/sim/movement/movement_bridge.rs](../../src/sim/movement/movement_bridge.rs) | Add `BridgeTransition` + `BridgeStateUpdate` enums; add `compute_bridge_transition` helper; rewrite `resolve_cell_transition_bridge_state` body and signature; rewrite `apply_pending_bridge_render_state` body and signature; delete `apply_bridge_lookahead_if_needed` |
| Modify | [src/sim/movement/movement_step.rs](../../src/sim/movement/movement_step.rs) | Update caller at line 664 to new resolver signature (plumb src coords, drop layer override) |
| Modify | [src/sim/movement/movement_tick.rs](../../src/sim/movement/movement_tick.rs) | Update import line 37-39; update caller at line 608 (plumb src, drop layer override + drop explicit `on_bridge = layer==Bridge`); update callers at 669 and 786 to new `apply_pending_bridge_render_state` signature; delete call site at 806 |
| Modify | [src/sim/pathfinding/core.rs](../../src/sim/pathfinding/core.rs) | `compute_neighbor_height` Case 3: change `(2..=4).contains(&diff)` to `diff == 4 && neighbor_cell.transition`; `astar_search` neighbor walkability (line 440): gate Ground→Bridge on `transition` |
| Modify | [src/sim/pathfinding/cell_entry.rs](../../src/sim/pathfinding/cell_entry.rs) | Update TODO(RE) comment at line 12-14 to reflect that bridge legality is now path-driven post-G2 fix |
| Modify | [src/sim/movement/movement_tests.rs](../../src/sim/movement/movement_tests.rs) | Add 3 integration tests for on_bridge timing at ramps (Tasks 17) |
| Modify | [src/sim/pathfinding/core_tests.rs](../../src/sim/pathfinding/core_tests.rs) | Add 5 A* regression tests for G3/G4 (Task 14) |

## Interface Changes

All interfaces are `pub(super)` within the `movement` module — no cross-module API impact.

| Function | Change |
|----------|--------|
| `compute_bridge_transition` | **NEW.** `fn compute_bridge_transition(src: &PathCell, dst: &PathCell) -> BridgeTransition` |
| `BridgeTransition` | **NEW enum.** `Enter { deck_level: u8 } \| Exit \| NoChange` |
| `BridgeStateUpdate` | **NEW enum.** `Set(u8) \| Clear \| Unchanged` |
| `resolve_cell_transition_bridge_state` | Signature change. New: `(position, path_grid, src: (u16,u16), dst: (u16,u16), next_layer) -> BridgeStateUpdate`. Drops the diag-id and diag-source params (unused for behavior). |
| `apply_pending_bridge_render_state` | Signature change. New param type: `bridge_update: BridgeStateUpdate` replaces `Option<Option<u8>>`. Body no longer sets `*on_bridge = active_layer == Bridge`. |
| `apply_bridge_lookahead_if_needed` | **DELETED** entirely along with `SHIP_HEIGHT_STEP` (only used internally to this function) |
| `HEIGHT_THRESHOLD` | **DELETED** (no longer used after predicate replaces heuristic) |
| `BRIDGE_Z_OFFSET` | **KEPT** — used by ship/water mover code outside this design's scope |

## Sim Checklist

- [x] All math uses signed `i8` arithmetic via `wrapping_sub` — no f32/f64 in game logic
- [x] No new state added to `BridgeOccupancy` / `on_bridge` — both already in deterministic state hash
- [x] No dependencies on render/ui/sidebar/audio/net introduced
- [x] Tick ordering: predicate runs at the SAME point as the current heuristic (after `move_entity`, before next-tick layer query). No ordering shift.
- [x] `BTreeMap<u64, GameEntity>` iteration order: not affected (no entity-set changes)

## Risk Areas

- **Replay divergence on bridge maps.** The fix changes when `on_bridge` toggles relative to the pre-fix code. Existing replays from before the fix will diverge. Acceptable — no published replays.
- **A* path shape changes.** Tightening Case 3 (G3+G4) makes A* reject paths through height-diff-2/3 cells and through bridge cells without `transition`. Paths that previously routed through those cells will now reroute or fail. Risk: a small number of existing tests may use pathological setups; update them as discovered.
- **Multiple call sites for the resolver and render-state functions.** All four sites must be updated in lockstep — one stale caller will break compilation. Tasks are ordered so all callers update before tests run.
- **Bump/scatter onto body cell from the side (off-path).** The cell-flag predicate handles this naturally: src=ground_cell, dst=body_cell, height_diff=4 (if cells happen to satisfy it) or NoChange. Either way, predicate-correct.

## Parity-Critical Items

| Task # | Item | Why it matters | Verification |
|--------|------|----------------|--------------|
| Task 2 | `compute_bridge_transition` entry condition is `dst_h == src_h.wrapping_sub(4) && dst.bridge_walkable` exactly | gamemd predicate is binary-verified literal (0x4B1812 SUB EAX,4). Off-by-one would produce wrong on_bridge timing every bridge crossing. | Task 3 unit tests (cases 1, 5, 7) + gamemd asm comparison |
| Task 2 | Exit condition is `!dst.bridge_walkable && src.bridge_walkable`, **independent** of entry (NOT else-branch) | Audit log 2026-05-11 caught the doc's pseudocode bug here; the bridge-ramp-to-cliff edge case requires independence | Task 3 unit test #6 (`cliff_drop_off_bridge_ramp`) |
| Task 2 | Height read as signed `i8` via `as i8 + wrapping_sub(4)` | Binary uses MOVSX (signed). u8 subtraction would underflow on malformed maps with height ≥ 128. | Task 3 unit test #7 (`signed_height_arithmetic`) |
| Task 6 | `on_bridge` driven by `BridgeStateUpdate`, NOT `active_layer == Bridge` | Decouples on_bridge from path-layer at ramps. Going up: ramp tick has loco.layer=Bridge but on_bridge=false (matches gamemd). | Task 7 tests #15/16/17 |
| Task 9 | `entity.on_bridge = resolved_layer == MovementLayer::Bridge` line at `movement_tick.rs:622` is **deleted** | Same parity issue as Task 6 in the drive_track-jump path | Manual code review; covered by integration test #23 |
| Task 14 | A* G3: `compute_neighbor_height` Case 3 requires `diff == 4 && neighbor_cell.transition` | gamemd CheckBridgeTraversal rejects Ground→Bridge without bridgehead. Fires every match where ground unit approaches a bridge from any cell other than the explicit bridgehead. | A* regression test #20 + #21 |
| Task 14 | A* G4: same condition (`diff == 4`) blocks diff 2 and 3 | gamemd CheckBridgeTraversal blocks 2/3 always. Visible on user-made stepped-terrain maps. | A* regression tests #18 + #19 |
| Task 15 | A* `astar_search` neighbor walkability also gates Ground→Bridge on `transition` | Defense in depth — Case 3 returns ground_level if diff≠4, but the walkability branch still needs to reject the Bridge-layer transition explicitly | A* regression test #21 |
| Task 17 | Integration tests assert on_bridge fires at Ramp→Body exactly, clears at Ramp→Ground exactly | The whole point of the design. Tests must pin the predicate timing tick-exact. | Tasks 23 + 24 in design's testing section |
| Task 20 | Manual in-game observation against gamemd on a bridge map | Final parity check. Player should not see Z-pops, layer flicker, or repath stutters at bridge approaches. | Side-by-side play vs gamemd.exe on a stock bridge map |

---

## Tasks

### Task 1: Add `BridgeTransition` and `BridgeStateUpdate` enums

**Why:** Foundation types. Every later task depends on these. Defining them first makes signatures clean.

**Files:**
- Modify: [src/sim/movement/movement_bridge.rs](../../src/sim/movement/movement_bridge.rs) (top of file, after `use` statements)

**Pattern:** matches the small-enum result types used in [src/sim/pathfinding/cell_entry.rs:39-59](../../src/sim/pathfinding/cell_entry.rs#L39-L59) (`CellEntryResult`, `TerrainCheckResult`).

**Step 1: Add the enums** after the existing `use` block in `movement_bridge.rs`. Place them between line 21 (last `use`) and line 23 (`const HEIGHT_THRESHOLD`):

```rust
/// Result of the gamemd on_bridge transition predicate at a cell boundary.
///
/// Two independent conditions (verified against gamemd Process_Drive_Track 0x4B0F20):
///   Enter: dst.height_level == src.height_level - 4 AND dst has bridge structural flag
///   Exit:  !(dst has bridge structural flag) AND src has bridge structural flag
/// Both conditions are mutually exclusive on retail data but evaluated
/// independently to match the binary exactly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum BridgeTransition {
    /// Unit just entered the bridge body deck. Set on_bridge=true, position.z=deck_level.
    Enter { deck_level: u8 },
    /// Unit just exited the bridge structure. Set on_bridge=false, position.z=dst.ground_level.
    Exit,
    /// No layer-state change at this transition.
    NoChange,
}

/// Bridge state update produced by `resolve_cell_transition_bridge_state`.
/// Drives `on_bridge` and `BridgeOccupancy` independently from `loco.layer`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum BridgeStateUpdate {
    /// on_bridge = true; bridge_occupancy = Some(BridgeOccupancy { deck_level })
    Set(u8),
    /// on_bridge = false; bridge_occupancy = None
    Clear,
    /// Leave on_bridge and bridge_occupancy unchanged
    Unchanged,
}
```

**Step 2: Verify compile**
Run: `cargo check -p ra2-rust-game 2>&1 | tail -20`
Expected: clean (no errors). Warnings about unused enums are fine — Task 2 consumes them.

**Step 3: Commit**
```
sim/movement_bridge: introduce BridgeTransition + BridgeStateUpdate enums
```

---

### Task 2: Add `compute_bridge_transition` pure-function predicate

**Why:** This is the gamemd-exact predicate, the load-bearing parity fix. Implementing it as a standalone pure function lets Task 3 unit-test it without touching movement_step / movement_tick.

**Files:**
- Modify: [src/sim/movement/movement_bridge.rs](../../src/sim/movement/movement_bridge.rs) (after the new enums from Task 1)

**Pattern:** pure-fn helper, matches [bump_crush::collect_crush_victims](../../src/sim/movement/bump_crush.rs) signature style — takes `&PathCell` references, returns a small enum.

**Step 1: Add the function** immediately after the `BridgeStateUpdate` enum, before `BRIDGE_Z_OFFSET`:

```rust
/// gamemd's on_bridge cell-flag predicate at a cell-boundary crossing.
///
/// Verified at Process_Drive_Track 0x4B0F20 / asm 0x4B1812 (entry) / 0x4B1830-184A (exit).
/// Both conditions evaluate independently — they are mutually exclusive on retail data
/// but the audit caught a doc pseudocode bug that incorrectly structured them as if/else.
/// See AUDIT_LOG.md 2026-05-11 entry on BRIDGE_SYSTEM.md §"Bridge Ramp Detection".
///
/// Height arithmetic uses signed i8 via wrapping_sub to match the binary's MOVSX read of
/// cell+0x11B. u8 subtraction would underflow on malformed maps with height ≥ 128;
/// retail maps use 0-15 only, so wrapping is functionally identical in practice.
pub(super) fn compute_bridge_transition(src: &PathCell, dst: &PathCell) -> BridgeTransition {
    let src_h = src.ground_level as i8;
    let dst_h = dst.ground_level as i8;

    let entry = dst_h == src_h.wrapping_sub(4) && dst.bridge_walkable;
    let exit = !dst.bridge_walkable && src.bridge_walkable;

    if entry {
        return BridgeTransition::Enter {
            deck_level: dst.bridge_deck_level_if_any().unwrap_or(dst.ground_level),
        };
    }
    if exit {
        return BridgeTransition::Exit;
    }
    BridgeTransition::NoChange
}
```

**Step 2: Verify compile**
Run: `cargo check -p ra2-rust-game 2>&1 | tail -20`
Expected: clean.

**Step 3: Commit**
```
sim/movement_bridge: add compute_bridge_transition cell-flag predicate
```

---

### Task 3: Unit tests for `compute_bridge_transition`

**Why:** Lock the predicate's behavior on every retail case before any caller depends on it. Eight tests pin the parity invariants.

**Files:**
- Modify: [src/sim/movement/movement_bridge.rs](../../src/sim/movement/movement_bridge.rs) (add `#[cfg(test)] mod tests` at end of file)

**Pattern:** matches the in-file test mod in [cell_entry.rs:282-432](../../src/sim/pathfinding/cell_entry.rs#L282-L432).

**Step 1: Add the test module** at the end of the file (after the last function):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::pathfinding::PathCell;

    /// Construct a synthetic PathCell with the bridge fields we care about.
    fn cell(ground_level: u8, bridge_walkable: bool, transition: bool) -> PathCell {
        let bridge_deck_level = if bridge_walkable { ground_level.saturating_add(4) } else { 0 };
        PathCell {
            ground_walkable: true,
            bridge_walkable,
            transition,
            ground_level,
            bridge_deck_level,
        }
    }

    #[test]
    fn entry_from_ramp_to_body() {
        // Ramp at height 4 (bridge_walkable, transition=true) → Body at height 0 (bridge_walkable, no transition).
        let src = cell(4, true, true);
        let dst = cell(0, true, false);
        match compute_bridge_transition(&src, &dst) {
            BridgeTransition::Enter { deck_level } => assert_eq!(deck_level, 4),
            other => panic!("expected Enter, got {:?}", other),
        }
    }

    #[test]
    fn exit_from_body_to_ground() {
        // Body at height 0 (bridge_walkable) → Ground at height 0 (NOT bridge_walkable).
        let src = cell(0, true, false);
        let dst = cell(0, false, false);
        assert_eq!(compute_bridge_transition(&src, &dst), BridgeTransition::Exit);
    }

    #[test]
    fn body_to_body_no_change() {
        let src = cell(0, true, false);
        let dst = cell(0, true, false);
        assert_eq!(compute_bridge_transition(&src, &dst), BridgeTransition::NoChange);
    }

    #[test]
    fn ground_to_ground_no_change() {
        let src = cell(0, false, false);
        let dst = cell(0, false, false);
        assert_eq!(compute_bridge_transition(&src, &dst), BridgeTransition::NoChange);
    }

    #[test]
    fn ground_to_bridgehead_no_change() {
        // Going UP onto a ramp is NOT an on_bridge transition: dst is HIGHER than src.
        // src=ground 0, dst=ramp 4. dst_h(4) == src_h(0) - 4 (=-4)? No. Entry doesn't fire.
        // src has no 0x100; exit doesn't fire. NoChange.
        let src = cell(0, false, false);
        let dst = cell(4, true, true);
        assert_eq!(compute_bridge_transition(&src, &dst), BridgeTransition::NoChange);
    }

    #[test]
    fn cliff_drop_off_bridge_ramp() {
        // Edge case from AUDIT_LOG 2026-05-11: src=ramp (h=4, bridge_walkable, transition),
        // dst=ground at lower elevation (h=0, NOT bridge_walkable). Height-diff matches 4
        // AND exit condition fires (!dst.bridge_walkable && src.bridge_walkable).
        // Exit precedence: predicate produces Exit (since entry needs dst.bridge_walkable=true).
        let src = cell(4, true, true);
        let dst = cell(0, false, false);
        assert_eq!(compute_bridge_transition(&src, &dst), BridgeTransition::Exit);
    }

    #[test]
    fn signed_height_arithmetic() {
        // Verify wrapping_sub handles the i8 boundary. src.ground_level = 4 (as i8 = 4),
        // dst.ground_level = 0 (as i8 = 0). 4.wrapping_sub(4) == 0. Entry should fire.
        let src = cell(4, true, true);
        let dst = cell(0, true, false);
        assert!(matches!(
            compute_bridge_transition(&src, &dst),
            BridgeTransition::Enter { deck_level: 4 }
        ));
    }

    #[test]
    fn entry_without_bridge_walkable_no_change() {
        // Height-diff matches 4 but dst is NOT bridge_walkable. Entry must NOT fire.
        let src = cell(4, false, false);
        let dst = cell(0, false, false);
        assert_eq!(compute_bridge_transition(&src, &dst), BridgeTransition::NoChange);
    }
}
```

**Step 2: Run tests**
Run: `cargo test -p ra2-rust-game --lib movement_bridge::tests -- --nocapture`
Expected: 8 tests PASS.

**Step 3: Commit**
```
sim/movement_bridge: 8 unit tests for compute_bridge_transition
```

---

### Task 4: Rewrite `resolve_cell_transition_bridge_state`

**Why:** Replace the height-based heuristic body with the predicate. Drop the layer return (caller continues to use path_layers). Plumb `src` coords so the predicate can read both cells.

**Files:**
- Modify: [src/sim/movement/movement_bridge.rs](../../src/sim/movement/movement_bridge.rs) — replace function at lines 37-77

**Pattern:** thin adapter around the pure-fn predicate; mirrors the structure of `bump_crush::cell_passable_after_crush` (orchestrates pure helpers).

**Step 1: Replace the function body and signature**. Find the current function (currently at lines 37-77, starts with `/// Resolve bridge layer state at a cell boundary crossing using reactive height\n/// comparison.`) and replace ENTIRELY with:

```rust
/// Apply gamemd's on_bridge cell-flag predicate at a cell-boundary crossing.
///
/// Reads src and dst PathCells and computes the BridgeStateUpdate using
/// `compute_bridge_transition`. Writes `position.z` to the post-transition height
/// (deck_level on Enter, dst.ground_level on Exit, or next_layer's effective height
/// on NoChange).
///
/// Does NOT return a layer — the caller continues to use `next_layer` from A*'s
/// `path_layers` for `loco.layer`. The predicate's role is independent: it drives
/// `on_bridge` and `BridgeOccupancy` via the returned `BridgeStateUpdate`.
///
/// Fallback: returns `Unchanged` (no position.z modification) when `path_grid` is
/// `None` or either cell lookup is out-of-bounds. Out-of-bounds at the boundary
/// crossing indicates a path-data bug elsewhere; the resolver is not the recovery point.
pub(super) fn resolve_cell_transition_bridge_state(
    position: &mut Position,
    path_grid: Option<&PathGrid>,
    src: (u16, u16),
    dst: (u16, u16),
    next_layer: MovementLayer,
) -> BridgeStateUpdate {
    let Some(grid) = path_grid else {
        return BridgeStateUpdate::Unchanged;
    };
    let (Some(src_cell), Some(dst_cell)) = (grid.cell(src.0, src.1), grid.cell(dst.0, dst.1))
    else {
        return BridgeStateUpdate::Unchanged;
    };

    match compute_bridge_transition(src_cell, dst_cell) {
        BridgeTransition::Enter { deck_level } => {
            position.z = deck_level;
            BridgeStateUpdate::Set(deck_level)
        }
        BridgeTransition::Exit => {
            position.z = dst_cell.ground_level;
            BridgeStateUpdate::Clear
        }
        BridgeTransition::NoChange => {
            position.z = dst_cell.effective_cell_z_for_layer(next_layer);
            BridgeStateUpdate::Unchanged
        }
    }
}
```

**Step 2: Verify compile**
Run: `cargo check -p ra2-rust-game 2>&1 | tail -30`
Expected: errors at the two call sites (`movement_step.rs:664` and `movement_tick.rs:608`) due to signature change. Those are fixed in Tasks 8 and 9. Do NOT fix them yet — Task 5 still adds tests against the new signature.

**Step 3: Commit**
```
sim/movement_bridge: rewrite resolve_cell_transition_bridge_state to predicate-driven

Replaces the reactive Z-based heuristic with a thin adapter over
compute_bridge_transition. New signature drops the layer return; on_bridge
is now decoupled from loco.layer. Caller sites updated in follow-up commits.
```

---

### Task 5: Unit tests for `resolve_cell_transition_bridge_state`

**Why:** Lock the resolver's behavior with the new signature before updating callers.

**Files:**
- Modify: [src/sim/movement/movement_bridge.rs](../../src/sim/movement/movement_bridge.rs) — extend the test module from Task 3

**Step 1: Add these tests** to the existing `#[cfg(test)] mod tests` block, after the predicate tests:

```rust
    use crate::sim::components::Position;
    use crate::sim::pathfinding::PathGrid;

    fn make_grid_with_cells(cells: &[(u16, u16, u8, bool, bool)]) -> PathGrid {
        let mut g = PathGrid::new(16, 16);
        for &(x, y, ground_level, bridge_walkable, transition) in cells {
            g.set_cell_for_test(x, y, ground_level, bridge_walkable, transition);
        }
        g
    }

    fn pos_at(rx: u16, ry: u16, z: u8) -> Position {
        Position {
            rx,
            ry,
            z,
            sub_x: crate::util::fixed_math::SimFixed::ZERO,
            sub_y: crate::util::fixed_math::SimFixed::ZERO,
            // screen_x/screen_y are #[serde(skip, default)] but Position has no
            // Default impl, so we must initialize them explicitly in struct literals.
            screen_x: 0.0,
            screen_y: 0.0,
        }
    }

    #[test]
    fn resolver_fallback_when_path_grid_none() {
        let mut p = pos_at(5, 5, 10);
        let update = resolve_cell_transition_bridge_state(
            &mut p,
            None,
            (5, 5),
            (6, 5),
            MovementLayer::Ground,
        );
        assert_eq!(update, BridgeStateUpdate::Unchanged);
        assert_eq!(p.z, 10, "position.z must be untouched on Unchanged");
    }

    #[test]
    fn resolver_fallback_when_cell_out_of_bounds() {
        let g = make_grid_with_cells(&[]);
        let mut p = pos_at(0, 0, 10);
        // src in bounds, dst out of bounds:
        let update = resolve_cell_transition_bridge_state(
            &mut p,
            Some(&g),
            (0, 0),
            (999, 999),
            MovementLayer::Ground,
        );
        assert_eq!(update, BridgeStateUpdate::Unchanged);
        assert_eq!(p.z, 10);
    }

    #[test]
    fn resolver_enter_writes_deck_level_and_set() {
        // src=ramp at h=4, dst=body at h=0 with bridge_walkable
        let g = make_grid_with_cells(&[
            (5, 5, 4, true, true),   // ramp
            (6, 5, 0, true, false),  // body
        ]);
        let mut p = pos_at(6, 5, 4);
        let update = resolve_cell_transition_bridge_state(
            &mut p,
            Some(&g),
            (5, 5),
            (6, 5),
            MovementLayer::Bridge,
        );
        assert_eq!(update, BridgeStateUpdate::Set(4));
        assert_eq!(p.z, 4, "position.z must equal deck_level on Enter");
    }

    #[test]
    fn resolver_exit_writes_ground_level_and_clear() {
        // src=body at h=0 bridge_walkable, dst=ground at h=0 NOT bridge_walkable
        let g = make_grid_with_cells(&[
            (5, 5, 0, true, false),
            (6, 5, 0, false, false),
        ]);
        let mut p = pos_at(6, 5, 4);
        let update = resolve_cell_transition_bridge_state(
            &mut p,
            Some(&g),
            (5, 5),
            (6, 5),
            MovementLayer::Ground,
        );
        assert_eq!(update, BridgeStateUpdate::Clear);
        assert_eq!(p.z, 0, "position.z must equal dst.ground_level on Exit");
    }

    #[test]
    fn resolver_no_change_with_next_layer_bridge_uses_deck() {
        // Body-to-body. NoChange. next_layer=Bridge → position.z = dst.bridge_deck_level (4).
        let g = make_grid_with_cells(&[
            (5, 5, 0, true, false),
            (6, 5, 0, true, false),
        ]);
        let mut p = pos_at(6, 5, 0);
        let update = resolve_cell_transition_bridge_state(
            &mut p,
            Some(&g),
            (5, 5),
            (6, 5),
            MovementLayer::Bridge,
        );
        assert_eq!(update, BridgeStateUpdate::Unchanged);
        assert_eq!(p.z, 4, "NoChange with next_layer=Bridge must use bridge_deck_level");
    }

    #[test]
    fn resolver_no_change_with_next_layer_ground_uses_ground() {
        // Ground-to-ground. NoChange. next_layer=Ground → position.z = dst.ground_level (0).
        let g = make_grid_with_cells(&[
            (5, 5, 0, false, false),
            (6, 5, 0, false, false),
        ]);
        let mut p = pos_at(6, 5, 0);
        let update = resolve_cell_transition_bridge_state(
            &mut p,
            Some(&g),
            (5, 5),
            (6, 5),
            MovementLayer::Ground,
        );
        assert_eq!(update, BridgeStateUpdate::Unchanged);
        assert_eq!(p.z, 0);
    }
```

**Step 2: Add the test helper `set_cell_for_test` to PathGrid.** This is a test-only method needed by the resolver tests above.

Open [src/sim/pathfinding/core.rs](../../src/sim/pathfinding/core.rs) and find the `impl PathGrid` block (search for `impl PathGrid {`, currently around line 727). Add this method at the end of the impl block (look for the closing brace of `impl PathGrid`):

```rust
    /// Test-only helper: directly write a cell's bridge fields.
    #[cfg(test)]
    pub fn set_cell_for_test(
        &mut self,
        x: u16,
        y: u16,
        ground_level: u8,
        bridge_walkable: bool,
        transition: bool,
    ) {
        if x < self.width && y < self.height {
            let idx = y as usize * self.width as usize + x as usize;
            let bridge_deck_level = if bridge_walkable {
                ground_level.saturating_add(4)
            } else {
                0
            };
            self.cells[idx] = PathCell {
                ground_walkable: true,
                bridge_walkable,
                transition,
                ground_level,
                bridge_deck_level,
            };
        }
    }
```

**Step 3: Run tests**
Run: `cargo test -p ra2-rust-game --lib movement_bridge::tests::resolver -- --nocapture`
Expected: 6 resolver tests PASS. (The 8 predicate tests from Task 3 also still pass.)

**Step 4: Commit**
```
sim/movement_bridge: 6 unit tests for resolve_cell_transition_bridge_state

Adds PathGrid::set_cell_for_test test helper.
```

---

### Task 6: Decouple `on_bridge` from `loco.layer` in `apply_pending_bridge_render_state`

**Why:** This is the second G2 parity bug — current code sets `*on_bridge = active_layer == Bridge`, conflating two distinct gamemd concepts. Decouple them: `loco.layer` follows A*'s path layer (unchanged), `on_bridge` follows the predicate's BridgeStateUpdate.

**Files:**
- Modify: [src/sim/movement/movement_bridge.rs](../../src/sim/movement/movement_bridge.rs) — replace function body at lines 79-101

**Step 1: Replace the function** (currently at lines 79-101) ENTIRELY with:

```rust
/// Apply the post-resolver bridge state to entity components.
///
/// `loco.layer` follows `active_layer` (= A*'s path_layer for this step), which drives
/// walkability and cell_entry occupancy lookup.
///
/// `on_bridge` and `bridge_occupancy` are driven INDEPENDENTLY by `bridge_update` from
/// the cell-flag predicate. This is the load-bearing G2 parity fix: gamemd's
/// on_bridge state (`FootClass+0x79` analog) is NOT derivable from the A* layer,
/// because on a ramp going up loco.layer=Bridge but on_bridge=false (predicate hasn't
/// fired Enter yet), and on a ramp going down loco.layer=Ground but on_bridge=true.
pub(super) fn apply_pending_bridge_render_state(
    locomotor: &mut Option<LocomotorState>,
    bridge_occupancy: &mut Option<BridgeOccupancy>,
    on_bridge: &mut bool,
    active_layer: MovementLayer,
    bridge_update: BridgeStateUpdate,
    _diag_entity_id: u64,
) {
    if let Some(loco) = locomotor {
        loco.layer = active_layer;
    }
    match bridge_update {
        BridgeStateUpdate::Set(deck_level) => {
            *on_bridge = true;
            *bridge_occupancy = Some(BridgeOccupancy { deck_level });
        }
        BridgeStateUpdate::Clear => {
            *on_bridge = false;
            *bridge_occupancy = None;
        }
        BridgeStateUpdate::Unchanged => {
            // on_bridge and bridge_occupancy retain their previous values
        }
    }
}
```

**Step 2: Verify compile**
Run: `cargo check -p ra2-rust-game 2>&1 | tail -30`
Expected: still has errors at the 2 caller sites for `apply_pending_bridge_render_state` (`movement_tick.rs:669` and `:786`) plus the 2 resolver call sites. All fixed in Tasks 8-11.

**Step 3: Commit**
```
sim/movement_bridge: decouple on_bridge from loco.layer in render-state apply

on_bridge and bridge_occupancy are now driven by BridgeStateUpdate (predicate
result), not by active_layer. Fixes the ramp-timing parity bug where on_bridge
flickered wrong by one tick at every ramp crossing.
```

---

### Task 7: Decoupling tests for `apply_pending_bridge_render_state`

**Why:** Regression guard against re-introducing the `*on_bridge = active_layer == Bridge` line. Pin the ramp timing semantics.

**Files:**
- Modify: [src/sim/movement/movement_bridge.rs](../../src/sim/movement/movement_bridge.rs) — extend the test module

**Step 1: Add these tests** to the test module, after the resolver tests from Task 5:

```rust
    use crate::sim::components::BridgeOccupancy;
    use crate::sim::movement::locomotor::{
        AirMovePhase, GroundMovePhase, LocomotorState,
    };
    use crate::rules::locomotor_type::{LocomotorKind, MovementZone, SpeedType};
    use crate::util::fixed_math::{SIM_ONE, SIM_ZERO};

    /// Build a minimal `LocomotorState` for tests. Mirrors the repo pattern
    /// at [src/sim/movement/droppod_movement.rs:205-234](make_walk_loco) — list
    /// all 27 fields explicitly with sensible defaults, since `LocomotorState`
    /// has no `Default` impl and `GameEntity::test_default` populates
    /// `locomotor: None`.
    fn make_loco(layer: MovementLayer) -> Option<LocomotorState> {
        Some(LocomotorState {
            kind: LocomotorKind::Drive,
            layer,
            phase: GroundMovePhase::Idle,
            air_phase: AirMovePhase::Landed,
            speed_multiplier: SIM_ONE,
            speed_fraction: SIM_ONE,
            fly_current_speed: SIM_ZERO,
            altitude: SIM_ZERO,
            target_altitude: SIM_ZERO,
            climb_rate: SIM_ZERO,
            jumpjet_speed: SIM_ZERO,
            jumpjet_wobbles: 0.0,
            jumpjet_accel: SIM_ZERO,
            jumpjet_current_speed: SIM_ZERO,
            jumpjet_deviation: 0,
            jumpjet_crash_speed: SIM_ZERO,
            jumpjet_turn_rate: 4,
            balloon_hover: false,
            hover_attack: false,
            speed_type: SpeedType::Track,
            movement_zone: MovementZone::Normal,
            rot: 0,
            override_state: None,
            air_progress: SIM_ZERO,
            infantry_wobble_phase: 0.0,
            subcell_dest: None,
        })
    }

    #[test]
    fn render_state_on_bridge_decoupled_from_loco_layer() {
        // active_layer=Bridge but bridge_update=Unchanged.
        // on_bridge must retain its prior value (does NOT become true just because layer is Bridge).
        let mut loco = make_loco(MovementLayer::Ground);
        let mut occ: Option<BridgeOccupancy> = None;
        let mut on_b = false;
        apply_pending_bridge_render_state(
            &mut loco,
            &mut occ,
            &mut on_b,
            MovementLayer::Bridge,
            BridgeStateUpdate::Unchanged,
            42,
        );
        assert_eq!(loco.as_ref().unwrap().layer, MovementLayer::Bridge);
        assert!(!on_b, "on_bridge must NOT be derived from active_layer");
        assert!(occ.is_none(), "bridge_occupancy must be unchanged");
    }

    #[test]
    fn render_state_ramp_going_up_keeps_on_bridge_false() {
        // Going up onto a ramp: A*'s path puts the ramp on Bridge layer, but the
        // predicate doesn't fire Enter until Ramp→Body next tick. So this tick:
        //   active_layer = Bridge, bridge_update = Unchanged, on_bridge = false (prior).
        let mut loco = make_loco(MovementLayer::Ground);
        let mut occ: Option<BridgeOccupancy> = None;
        let mut on_b = false;
        apply_pending_bridge_render_state(
            &mut loco,
            &mut occ,
            &mut on_b,
            MovementLayer::Bridge,
            BridgeStateUpdate::Unchanged,
            42,
        );
        assert_eq!(loco.as_ref().unwrap().layer, MovementLayer::Bridge);
        assert!(!on_b, "on_bridge must stay false on the ramp tick going up");
    }

    #[test]
    fn render_state_ramp_going_down_keeps_on_bridge_true() {
        // Coming off a bridge: A*'s path puts the ramp on Ground layer (is_at_bridge_level
        // returns false), but the predicate hasn't fired Exit yet. on_bridge stays true.
        let mut loco = make_loco(MovementLayer::Bridge);
        let mut occ = Some(BridgeOccupancy { deck_level: 4 });
        let mut on_b = true;
        apply_pending_bridge_render_state(
            &mut loco,
            &mut occ,
            &mut on_b,
            MovementLayer::Ground,
            BridgeStateUpdate::Unchanged,
            42,
        );
        assert_eq!(loco.as_ref().unwrap().layer, MovementLayer::Ground);
        assert!(on_b, "on_bridge must stay true on the ramp tick going down");
        assert!(occ.is_some(), "bridge_occupancy must be unchanged on Unchanged");
    }

    #[test]
    fn render_state_set_writes_occupancy() {
        let mut loco = make_loco(MovementLayer::Bridge);
        let mut occ: Option<BridgeOccupancy> = None;
        let mut on_b = false;
        apply_pending_bridge_render_state(
            &mut loco,
            &mut occ,
            &mut on_b,
            MovementLayer::Bridge,
            BridgeStateUpdate::Set(4),
            42,
        );
        assert!(on_b);
        assert_eq!(occ.unwrap().deck_level, 4);
    }

    #[test]
    fn render_state_clear_drops_occupancy() {
        let mut loco = make_loco(MovementLayer::Ground);
        let mut occ = Some(BridgeOccupancy { deck_level: 4 });
        let mut on_b = true;
        apply_pending_bridge_render_state(
            &mut loco,
            &mut occ,
            &mut on_b,
            MovementLayer::Ground,
            BridgeStateUpdate::Clear,
            42,
        );
        assert!(!on_b);
        assert!(occ.is_none());
    }
```

**Step 2: Run tests**
Run: `cargo test -p ra2-rust-game --lib movement_bridge::tests::render_state -- --nocapture`
Expected: 5 tests PASS. (Note: this also runs together with the resolver and predicate tests as part of the same module.)

**Step 3: Commit**
```
sim/movement_bridge: 5 decoupling tests for apply_pending_bridge_render_state

Regression guard for the ramp-timing G2 parity fix: on_bridge must NOT be
derived from active_layer.
```

---

### Task 8: Update `movement_step.rs` call site to the new resolver signature

**Why:** Bring the main cell-crossing path in line with the new resolver signature. Plumb `(old_rx, old_ry)` as src; drop the layer override; drop the resolver's diag params.

**Files:**
- Modify: [src/sim/movement/movement_step.rs](../../src/sim/movement/movement_step.rs) — lines 661-678

**Step 1: Read the current code at lines 661-678** for context. The current code is:

```rust
        occupancy.move_entity(old_rx, old_ry, nx, ny, entity_id, active_layer, *sub_cell);
        // Bridge/layer resolution stays in one helper so cell transitions
        // don't duplicate deck/ground height rules across the tick loop.
        let (resolved_layer, bridge_update) = resolve_cell_transition_bridge_state(
            position,
            path_grid,
            next_layer,
            nx,
            ny,
            entity_id,
            "cell_crossing",
        );
        next_layer = resolved_layer;
        pending_bridge_update = bridge_update;
        active_layer = next_layer;
        if let Some(loco) = locomotor {
            loco.layer = next_layer;
        }
```

**Step 2: Replace those lines** (specifically the `let (resolved_layer, bridge_update) = ...` through the `if let Some(loco) = locomotor { loco.layer = next_layer; }`) with:

```rust
        occupancy.move_entity(old_rx, old_ry, nx, ny, entity_id, active_layer, *sub_cell);
        // Bridge state resolution: apply gamemd's on_bridge cell-flag predicate.
        // Returns ONLY a BridgeStateUpdate — loco.layer continues to follow
        // A*'s path_layers (next_layer was set at line 467).
        let bridge_update = resolve_cell_transition_bridge_state(
            position,
            path_grid,
            (old_rx, old_ry),
            (nx, ny),
            next_layer,
        );
        pending_bridge_update = bridge_update;
        active_layer = next_layer;
        if let Some(loco) = locomotor {
            loco.layer = next_layer;
        }
```

Key changes:
- Drops the `let (resolved_layer, ...)` destructure → `let bridge_update = ...`
- Adds `(old_rx, old_ry)` and `(nx, ny)` tuple args
- Removes the `entity_id, "cell_crossing"` diag args (not in the new signature)
- Removes `next_layer = resolved_layer;` line (resolver no longer returns a layer)

**Step 3: Verify compile**
Run: `cargo check -p ra2-rust-game 2>&1 | tail -30`
Expected: errors remain only at `movement_tick.rs:608`, `:669`, `:786`, `:806`. Fixed in Tasks 9-13.

**Step 4: Commit**
```
sim/movement_step: update bridge-resolver call site to new signature

Drops layer override (path_layers now authoritative for loco.layer) and
plumbs (old_rx, old_ry) as src for the predicate.
```

---

### Task 9: Update `movement_tick.rs` drive-track call site + delete the `entity.on_bridge = ...` bug

**Why:** Same signature update as Task 8, plus deletes the `entity.on_bridge = resolved_layer == MovementLayer::Bridge` line at 622 which is the same parity bug as Task 6.

**Files:**
- Modify: [src/sim/movement/movement_tick.rs](../../src/sim/movement/movement_tick.rs) — lines 605-623

**Step 1: Read current code at lines 605-623** for context. Current code is:

```rust
                        if let Some(pg) = path_grid {
                            let next_layer = target.layer_at(target.next_index);
                            let (resolved_layer, bridge_update) =
                                super::movement_bridge::resolve_cell_transition_bridge_state(
                                    &mut entity.position,
                                    Some(pg),
                                    next_layer,
                                    nx,
                                    ny,
                                    entity_id,
                                    "drive_track_jump",
                                );
                            pending_bridge_update = bridge_update;
                            active_layer = resolved_layer;
                            if let Some(ref mut loco) = entity.locomotor {
                                loco.layer = resolved_layer;
                            }
                            entity.on_bridge = resolved_layer == MovementLayer::Bridge;
                        }
```

**Step 2: Replace those lines** with:

```rust
                        if let Some(pg) = path_grid {
                            let next_layer = target.layer_at(target.next_index);
                            // Bridge state resolution: apply gamemd's on_bridge cell-flag predicate.
                            // loco.layer follows A*'s path_layer (next_layer). on_bridge will be
                            // updated by apply_pending_bridge_render_state from bridge_update below.
                            let bridge_update =
                                super::movement_bridge::resolve_cell_transition_bridge_state(
                                    &mut entity.position,
                                    Some(pg),
                                    (old_rx, old_ry),
                                    (nx, ny),
                                    next_layer,
                                );
                            pending_bridge_update = bridge_update;
                            active_layer = next_layer;
                            if let Some(ref mut loco) = entity.locomotor {
                                loco.layer = next_layer;
                            }
                            // NOTE: entity.on_bridge is now driven by the predicate via
                            // apply_pending_bridge_render_state, NOT by the layer match.
                            // The previous `entity.on_bridge = resolved_layer == Bridge`
                            // was the G2 parity bug: it flickered on_bridge wrong by one
                            // tick at every ramp crossing.
                        }
```

Key changes:
- Drops `(resolved_layer, bridge_update)` destructure → `let bridge_update = ...`
- New args: `(old_rx, old_ry)` and `(nx, ny)` tuples
- Removes `entity_id, "drive_track_jump"` diag args
- `active_layer = next_layer` (not `resolved_layer`)
- `loco.layer = next_layer`
- **DELETES** the `entity.on_bridge = resolved_layer == MovementLayer::Bridge` line — replaced by a comment explaining why

**Step 3: Verify compile**
Run: `cargo check -p ra2-rust-game 2>&1 | tail -30`
Expected: errors remain at `movement_tick.rs:669`, `:786`, `:806`. Plus the `apply_pending_bridge_render_state` calls at 669/786 still have the old signature.

**Step 4: Commit**
```
sim/movement_tick: update drive-track bridge-resolver call site

Same signature update as movement_step. Also deletes the
`entity.on_bridge = resolved_layer == Bridge` line — the G2 parity bug.
on_bridge will be set correctly by apply_pending_bridge_render_state.
```

---

### Task 10: Thread `BridgeStateUpdate` through `CrossingOutput` and all callers

**Why:** The `pending_bridge_update` type change ripples through multiple sites:
- `CrossingOutput.pending_bridge_update` struct field at [movement_step.rs:393](../../src/sim/movement/movement_step.rs#L393) is `Option<Option<u8>>` — must change to `BridgeStateUpdate`.
- The local in `process_step_for_entity` at [movement_step.rs:439](../../src/sim/movement/movement_step.rs#L439) is `Option<Option<u8>>` — must change.
- `process_step_for_entity` constructs `CrossingOutput` near its return point ([movement_step.rs:744-750](../../src/sim/movement/movement_step.rs#L744-L750)) — that construction site forwards the local into the field; type now matches.
- In `movement_tick.rs`, the local at [movement_tick.rs:414](../../src/sim/movement/movement_tick.rs#L414) is `Option<Option<u8>>` — must change. The read from `crossing.pending_bridge_update` at [movement_tick.rs:773](../../src/sim/movement/movement_tick.rs#L773) and the use at [movement_tick.rs:617](../../src/sim/movement/movement_tick.rs#L617) become type-correct after the field change.

**Files:**
- Modify: [src/sim/movement/movement_step.rs](../../src/sim/movement/movement_step.rs) — lines 393, 439
- Modify: [src/sim/movement/movement_tick.rs](../../src/sim/movement/movement_tick.rs) — line 414, plus import line 37-39

**Step 1: Update `CrossingOutput.pending_bridge_update` field type.** In [src/sim/movement/movement_step.rs](../../src/sim/movement/movement_step.rs), find:

```rust
    /// Bridge render state to apply after the loop.
    pub pending_bridge_update: Option<Option<u8>>,
```

Replace with:

```rust
    /// Bridge render state to apply after the loop. Predicate-driven; see movement_bridge.rs.
    pub pending_bridge_update: super::movement_bridge::BridgeStateUpdate,
```

**Step 2: Update local declaration at line ~439 in movement_step.rs.** Find:

```rust
    let mut pending_bridge_update: Option<Option<u8>> = None;
```

Replace with:

```rust
    let mut pending_bridge_update: super::movement_bridge::BridgeStateUpdate =
        super::movement_bridge::BridgeStateUpdate::Unchanged;
```

**Step 3: Update local declaration in movement_tick.rs at line ~414.** Find:

```rust
        let mut pending_bridge_update: Option<Option<u8>> = None;
```

Replace with:

```rust
        let mut pending_bridge_update: super::movement_bridge::BridgeStateUpdate =
            super::movement_bridge::BridgeStateUpdate::Unchanged;
```

**Step 4: Add `BridgeStateUpdate` to the movement_tick.rs import.** Current import at lines 37-39:

```rust
use super::movement_bridge::{
    BRIDGE_Z_OFFSET, apply_bridge_lookahead_if_needed, apply_pending_bridge_render_state,
};
```

(Task 13 will remove `apply_bridge_lookahead_if_needed`. For now, just add `BridgeStateUpdate`:)

```rust
use super::movement_bridge::{
    BRIDGE_Z_OFFSET, BridgeStateUpdate, apply_bridge_lookahead_if_needed,
    apply_pending_bridge_render_state,
};
```

This lets later edits write `BridgeStateUpdate::Unchanged` without the `super::movement_bridge::` prefix.

**Step 5: Search for any remaining `Option<Option<u8>>` patterns** and convert per the mapping below. Run:

```
grep -n "pending_bridge_update" src/sim/movement/movement_tick.rs src/sim/movement/movement_step.rs
```

For any pattern-match or value-construction sites that still use the old type:
- `None` (as a `pending_bridge_update` value) → `BridgeStateUpdate::Unchanged`
- `Some(None)` → `BridgeStateUpdate::Clear`
- `Some(Some(level))` → `BridgeStateUpdate::Set(level)`
- `Option<Option<u8>>` in pattern matches → match `BridgeStateUpdate { Unchanged | Clear | Set(u8) }`

In practice, after Tasks 4 and 6 the resolver and render-state-apply already return/accept `BridgeStateUpdate`, so the value flow is clean. The local-type and field-type changes in Steps 1-3 should be sufficient.

**Step 6: Verify compile**
Run: `cargo check -p ra2-rust-game 2>&1 | tail -30`
Expected: errors only at the remaining `apply_pending_bridge_render_state` call site that still has the old signature (movement_tick.rs:786/791 — Task 11) and the lookahead call (movement_tick.rs:806 — Task 13).

**Step 7: Commit**
```
sim/movement: thread BridgeStateUpdate through CrossingOutput + drive-track render

Updates CrossingOutput.pending_bridge_update field type and the local
variable declarations in process_step_for_entity and movement_tick.
```

---

### Task 11: Update `movement_tick.rs:786` to new `apply_pending_bridge_render_state` signature

**Why:** Second render-state apply call site, in the main tick path.

**Files:**
- Modify: [src/sim/movement/movement_tick.rs](../../src/sim/movement/movement_tick.rs) — around line 786

**Step 1: Read current code at lines 783-790** for context. Current call is:

```rust
            if !aborted_for_stuck
                && !matches!(deferred_cell_check, Some(DeferredCellCheck::Vehicle(_, _)))
            {
                apply_pending_bridge_render_state(
                    &mut entity.locomotor,
                    &mut entity.bridge_occupancy,
                    &mut entity.on_bridge,
                    [active_layer],
                    [pending_bridge_update],
                    entity_id,
                );
            }
```

(The `[brackets]` are placeholders for whatever the current local var names are at that scope — confirm by reading the file.)

**Step 2: Confirm the call passes `BridgeStateUpdate` (not `Option<Option<u8>>`).** After Task 10, the `pending_bridge_update` local in this scope should already be `BridgeStateUpdate`. No call-site change needed if the local var is the same one declared in Task 10.

**Step 3: If the call site at line 786 uses a DIFFERENT local variable** (different function scope), find its declaration and convert it the same way as Task 10 Step 3. Then verify the call site uses the new type.

**Step 4: Verify compile**
Run: `cargo check -p ra2-rust-game 2>&1 | tail -30`
Expected: errors only at `movement_tick.rs:806` (lookahead delete).

**Step 5: Commit**
```
sim/movement_tick: update main-path render-state apply for BridgeStateUpdate
```

---

### Task 12: Delete `apply_bridge_lookahead_if_needed` function

**Why:** This anticipatory layer-change is a workaround for the broken primary mechanism. gamemd has no anticipatory layer change. Now that the predicate is correct, delete the workaround.

**Files:**
- Modify: [src/sim/movement/movement_bridge.rs](../../src/sim/movement/movement_bridge.rs) — delete function at lines 103-147 plus the `SHIP_HEIGHT_STEP` constant at lines 27-30 plus the `HEIGHT_THRESHOLD` constant at lines 23-25

**Step 1: Delete the `HEIGHT_THRESHOLD` constant** at lines 23-25:

```rust
/// Threshold for ground vs bridge level detection.
/// If `abs(unit_z - cell.ground_level) >= HEIGHT_THRESHOLD`, unit is at bridge level.
const HEIGHT_THRESHOLD: u8 = 2;
```

**Step 2: Delete the `SHIP_HEIGHT_STEP` constant** at lines 27-30 IF NOT USED ELSEWHERE. First verify:

```
grep -rn "SHIP_HEIGHT_STEP" src/
```

If only the lookahead function uses it (and the comment for `BRIDGE_Z_OFFSET` references it but doesn't import it), delete:

```rust
/// Height of one ship Z-step in leptons.
/// Computed as `ftol(sin(30 deg) * 256*sqrt(2) * 0.5) = 90`.
#[allow(dead_code)]
pub(super) const SHIP_HEIGHT_STEP: SimFixed = SimFixed::lit("90");
```

If a `grep` shows OTHER uses, KEEP the constant.

**Step 3: Delete the function `apply_bridge_lookahead_if_needed`** at lines 103-147 entirely (the whole `/// Preemptive bridge detection ...` block through the closing `}`).

**Step 4: Update the module doc comment at lines 1-15** to reflect that the heuristic is gone:

Replace:

```rust
//! Bridge layer transitions — resolves ground-to-bridge and bridge-to-ground layer changes
//! during cell boundary crossings, and applies bridge render state for smooth visual transitions.
//!
//! Uses **reactive height-based detection**:
//! - `abs(unit_z - cell.ground_level) >= 2` → unit is at bridge level → stay on bridge
//! - `abs(unit_z - cell.ground_level) < 2` → unit is at ground level → pass under
//! - Ramp entry: `src_z == dst_ground + 4` with bridge flag → going UP onto bridge
//! Path layers are NOT used for bridge state decisions; the unit's Z relative to the
//! cell's ground height determines everything at runtime.
//!
//! TODO(RE): The stock game keeps explicit bridge-layer state on the unit
//! (`FootClass+0x79`) and feeds that into bridge-aware zone lookups. This module still
//! infers bridge state from reactive height heuristics and ignores the pathfinder's
//! `_next_layer` hints. Keep this conservative until the runtime bridge-layer update
//! rules are fully wired in.
```

With:

```rust
//! Bridge layer transitions — applies gamemd's on_bridge cell-flag predicate at each
//! cell boundary crossing, and decouples `on_bridge` from `loco.layer` so that the
//! A* path layer (walkability-driving) and the runtime bridge state (predicate-driven)
//! can disagree at ramp cells — which they must, to match gamemd.
//!
//! Predicate (verified at gamemd Process_Drive_Track 0x4B0F20):
//!   Enter:  dst.height_level == src.height_level - 4 AND dst.flags & 0x100
//!   Exit:   !(dst.flags & 0x100) AND src.flags & 0x100
//! Both conditions independent; signed i8 height arithmetic.
//!
//! See docs/plans/2026-05-11-bridge-locomotor-layer-correctness-design.md.
```

**Step 5: Verify compile**
Run: `cargo check -p ra2-rust-game 2>&1 | tail -30`
Expected: error remains only at `movement_tick.rs:806` (call site for the now-deleted function).

**Step 6: Commit**
```
sim/movement_bridge: delete apply_bridge_lookahead_if_needed and heuristic constants

The anticipatory layer change was a workaround for the broken reactive
heuristic. Now that the gamemd cell-flag predicate is in place, the
lookahead has no role: gamemd makes layer transitions at the cell boundary
exactly, never anticipatorily.
```

---

### Task 13: Remove `apply_bridge_lookahead_if_needed` call site in `movement_tick.rs`

**Why:** The function is gone; the call site at line 806 must go too. The surrounding code computes a `lookahead_layer` local that's now unused.

**Files:**
- Modify: [src/sim/movement/movement_tick.rs](../../src/sim/movement/movement_tick.rs) — line 37-39 import, lines 803-820+ call site

**Step 1: Read current code at lines 803-820** for context:

```rust
            };
            let lookahead_layer = target.layer_at(target.next_index);
            apply_bridge_lookahead_if_needed(
                &mut entity.position,
                &mut entity.bridge_occupancy,
                &mut entity.on_bridge,
                <mover_zone>,
                <next_step>,
                lookahead_layer,
                path_grid,
            );
```

(The `<placeholders>` are what's actually at those lines — confirm by reading.)

**Step 2: Delete** the `let lookahead_layer = ...;` line and the entire `apply_bridge_lookahead_if_needed(...)` call. Leave a brief comment if helpful for future readers:

```rust
            };
            // (Removed apply_bridge_lookahead_if_needed call: anticipatory layer change
            // is a workaround for the broken reactive heuristic; gamemd makes layer
            // transitions at the cell boundary exactly, see movement_bridge.rs predicate.)
```

**Step 3: Update the `use` import at lines 37-39**. Current:

```rust
use super::movement_bridge::{
    BRIDGE_Z_OFFSET, apply_bridge_lookahead_if_needed, apply_pending_bridge_render_state,
};
```

Change to:

```rust
use super::movement_bridge::{BRIDGE_Z_OFFSET, apply_pending_bridge_render_state};
```

(If `BridgeStateUpdate` is also imported here per Task 10, keep it in the list.)

**Step 4: Verify compile**
Run: `cargo build -p ra2-rust-game 2>&1 | tail -20`
Expected: clean build, no errors.

**Step 5: Run movement tests**
Run: `cargo test -p ra2-rust-game --lib movement -- --nocapture 2>&1 | tail -30`
Expected: all PASS. (Predicate, resolver, decoupling tests from Tasks 3/5/7 + existing movement tests.)

**Step 6: Commit**
```
sim/movement_tick: drop apply_bridge_lookahead_if_needed call and import

Companion to the function deletion in movement_bridge.rs.
```

---

### Task 14: A* G4 fix — tighten `compute_neighbor_height` Case 3 to `diff == 4` AND require `transition`

**Why:** gamemd's CheckBridgeTraversal at 0x4D9C60 only accepts height-diff 4 (with bridgehead) for Ground→Bridge entry. Diffs 2/3/5+ are always blocked. Our A* currently accepts (2..=4) and ignores the bridgehead flag in Case 3.

**Files:**
- Modify: [src/sim/pathfinding/core.rs](../../src/sim/pathfinding/core.rs) — lines 144-152

**Step 1: Read current code at lines 144-152** for context:

```rust
    // Case 3: Parent is NOT bridge, neighbor IS bridge.
    // Ramp-up restricted to diff in [2, 4].
    let diff = parent_height as i16 - neighbor_cell.ground_level as i16;
    if (2..=4).contains(&diff) {
        neighbor_cell.bridge_deck_level
    } else {
        neighbor_cell.ground_level
    }
}
```

**Step 2: Replace with:**

```rust
    // Case 3: Parent is NOT bridge, neighbor IS bridge.
    // gamemd CheckBridgeTraversal (0x4D9C60): Ground→Bridge entry requires
    // height-diff EXACTLY 4 AND the bridgehead flag (transition = 0x200 analog).
    // Diffs 2/3/5+ are always blocked. Diff 0/1 fall to other cases.
    let diff = parent_height as i16 - neighbor_cell.ground_level as i16;
    if diff == 4 && neighbor_cell.transition {
        neighbor_cell.bridge_deck_level
    } else {
        neighbor_cell.ground_level
    }
}
```

**Step 3: Verify compile**
Run: `cargo check -p ra2-rust-game 2>&1 | tail -10`
Expected: clean.

**Step 4: Commit**
```
sim/pathfinding: A* G4 fix — Case 3 requires diff==4 AND bridgehead flag

Matches gamemd CheckBridgeTraversal (0x4D9C60): diff 2/3/5+ always blocked,
diff 4 requires the 0x200 bridgehead flag for Ground→Bridge entry.
```

---

### Task 15: A* G3 fix — gate Ground→Bridge crossings on `transition` flag in `astar_search`

**Why:** Defense in depth — Case 3 now returns `ground_level` for non-bridgehead bridge cells, but the layer-decision via `is_at_bridge_level` is separate. The walkability check must ALSO reject Ground→Bridge crossings without the bridgehead.

**Files:**
- Modify: [src/sim/pathfinding/core.rs](../../src/sim/pathfinding/core.rs) — lines 438-449

**Step 1: Read current code at lines 438-449** for context:

```rust
            // Walkability check on the determined layer
            let neighbor_passable = if neighbor_use_bridge {
                grid.is_walkable_on_layer(nx, ny, MovementLayer::Bridge)
            } else {
                is_cell_passable_for_mover(
                    grid,
                    nx,
                    ny,
                    options.movement_zone,
                    options.resolved_terrain,
                )
            };
```

**Step 2: Replace with:**

```rust
            // Walkability check on the determined layer.
            // gamemd's CheckBridgeTraversal also requires the bridgehead flag (transition)
            // when transitioning from ground to bridge layer. A unit already on the
            // bridge can move between any two bridge_walkable cells (body-to-body
            // diagonals); only the Ground→Bridge transition requires the bridgehead.
            let neighbor_passable = if neighbor_use_bridge {
                let prev_on_bridge = is_at_bridge_level(current.height, cur_cell);
                if prev_on_bridge {
                    grid.is_walkable_on_layer(nx, ny, MovementLayer::Bridge)
                } else {
                    grid.is_walkable_on_layer(nx, ny, MovementLayer::Bridge)
                        && neighbor_cell.transition
                }
            } else {
                is_cell_passable_for_mover(
                    grid,
                    nx,
                    ny,
                    options.movement_zone,
                    options.resolved_terrain,
                )
            };
```

**Step 3: Verify compile**
Run: `cargo check -p ra2-rust-game 2>&1 | tail -10`
Expected: clean.

**Step 4: Commit**
```
sim/pathfinding: A* G3 fix — gate Ground→Bridge crossings on bridgehead flag

Matches gamemd CheckBridgeTraversal: bridge-body cells (0x100 only, no 0x200)
cannot be entered from ground level. Body-to-body diagonals (unit already
on bridge) still allowed.
```

---

### Task 16: A* G3+G4 regression tests in `core_tests.rs`

**Why:** Lock the A* behavior change. 5 tests from the design.

**Files:**
- Modify: [src/sim/pathfinding/core_tests.rs](../../src/sim/pathfinding/core_tests.rs) — append to the existing test module

**Step 1: Find the end of the test module** in `core_tests.rs`. Locate the final closing `}` of the file's last test function.

**Step 2: Add these tests** just before the file's final `}` (if the file ends with `}` for a `mod tests` block) or as standalone `#[test]` fns if the file's top-level is `#[cfg(test)] mod tests { ... }`:

```rust
    use crate::sim::movement::locomotor::MovementLayer;
    use crate::sim::pathfinding::{AStarOptions, PathGrid, astar_search};

    fn make_grid_for_bridge_test() -> PathGrid {
        // 10x10 grid. Default cells are ground_walkable at height 0.
        PathGrid::new(10, 10)
    }

    #[test]
    fn astar_blocks_height_diff_2() {
        // Adjacent ground cells at heights 0 and 2, no bridge. Should NOT path through.
        let mut g = make_grid_for_bridge_test();
        // Path: (1,1) at h=0 → (2,1) at h=2 ground (no bridge).
        g.set_cell_for_test(2, 1, 2, false, false);
        let result = astar_search(
            &g,
            (1, 1),
            MovementLayer::Ground,
            (3, 1),
            &AStarOptions::default(),
        );
        // Either no path, or a path that detours around (2,1).
        if let Some(path) = result {
            assert!(
                !path.iter().any(|s| (s.rx, s.ry) == (2, 1)),
                "A* must not route through diff-2 ground step"
            );
        }
    }

    #[test]
    fn astar_blocks_height_diff_3() {
        let mut g = make_grid_for_bridge_test();
        g.set_cell_for_test(2, 1, 3, false, false);
        let result = astar_search(
            &g,
            (1, 1),
            MovementLayer::Ground,
            (3, 1),
            &AStarOptions::default(),
        );
        if let Some(path) = result {
            assert!(
                !path.iter().any(|s| (s.rx, s.ry) == (2, 1)),
                "A* must not route through diff-3 ground step"
            );
        }
    }

    #[test]
    fn astar_allows_height_diff_4_with_bridgehead() {
        // Path: ground (1,1) at h=0 → bridgehead (2,1) at h=0 with bridge_walkable+transition
        //       → body (3,1) at h=0 with bridge_walkable only.
        let mut g = make_grid_for_bridge_test();
        // Bridgehead at (2,1): height_level=0 (raw), bridge_walkable=true, transition=true.
        // For Case 3 to fire as diff==4, we need parent_height - neighbor.ground_level == 4.
        // That means parent must be elevated by 4 — i.e., the ground at (1,1) must be at h=4.
        // Set ground (1,1) to height 4; bridgehead (2,1) to ground_level=0, bridge_walkable, transition.
        g.set_cell_for_test(1, 1, 4, false, false);
        g.set_cell_for_test(2, 1, 0, true, true); // bridgehead
        g.set_cell_for_test(3, 1, 0, true, false); // body
        let result = astar_search(
            &g,
            (1, 1),
            MovementLayer::Ground,
            (3, 1),
            &AStarOptions::default(),
        );
        assert!(result.is_some(), "A* must find a path through bridgehead");
        let path = result.unwrap();
        // (2,1) should appear on the Bridge layer in the path.
        let step_2_1 = path.iter().find(|s| (s.rx, s.ry) == (2, 1));
        assert!(step_2_1.is_some(), "Path must include (2,1)");
        assert_eq!(
            step_2_1.unwrap().layer,
            MovementLayer::Bridge,
            "Bridgehead step must be on Bridge layer (G3)"
        );
    }

    #[test]
    fn astar_blocks_height_diff_4_without_bridgehead() {
        // Same as above but (2,1) has transition=false (body cell, not bridgehead).
        // A* must NOT route Ground→Bridge through this cell.
        let mut g = make_grid_for_bridge_test();
        g.set_cell_for_test(1, 1, 4, false, false);
        g.set_cell_for_test(2, 1, 0, true, false); // body, no transition
        g.set_cell_for_test(3, 1, 0, true, false);
        let result = astar_search(
            &g,
            (1, 1),
            MovementLayer::Ground,
            (3, 1),
            &AStarOptions::default(),
        );
        // If A* finds a path, it must NOT go Ground→Bridge through (2,1).
        if let Some(path) = result {
            let through_body = path
                .iter()
                .find(|s| (s.rx, s.ry) == (2, 1) && s.layer == MovementLayer::Bridge);
            assert!(
                through_body.is_none(),
                "A* must not route Ground→Bridge through body cell without bridgehead (G3)"
            );
        }
    }

    #[test]
    fn astar_allows_body_to_body_diagonal() {
        // Two adjacent body cells (bridge_walkable, no transition). Unit already on
        // bridge can move between them. Regression: G3 fix must NOT over-tighten.
        let mut g = make_grid_for_bridge_test();
        // Start on a bridgehead so the unit enters the bridge.
        g.set_cell_for_test(0, 1, 4, false, false); // ground at h=4
        g.set_cell_for_test(1, 1, 0, true, true);   // bridgehead
        g.set_cell_for_test(2, 1, 0, true, false);  // body
        g.set_cell_for_test(2, 2, 0, true, false);  // body diagonal
        g.set_cell_for_test(3, 2, 0, true, true);   // exit bridgehead
        g.set_cell_for_test(4, 2, 4, false, false); // ground at h=4
        let result = astar_search(
            &g,
            (0, 1),
            MovementLayer::Ground,
            (4, 2),
            &AStarOptions::default(),
        );
        assert!(result.is_some(), "A* must find a path across the bridge");
        // No assertion on specific layer per step — just that a path exists.
    }
```

**Step 3: Run tests**
Run: `cargo test -p ra2-rust-game --lib pathfinding -- --nocapture 2>&1 | tail -30`
Expected: 5 new tests PASS plus all existing pathfinding tests PASS.

**Step 4: Commit**
```
sim/pathfinding: 5 A* regression tests for G3 + G4 fixes

Pins: diff-2 and diff-3 blocked, diff-4 with bridgehead allowed, diff-4 without
bridgehead rejected, body-to-body diagonals still allowed.
```

---

### Task 17: Movement integration tests for on_bridge timing at ramps

**Why:** Verify the end-to-end behavior: on_bridge fires at Ramp→Body exactly, clears at Ramp→Ground exactly. Tick-exact predicate timing.

**Files:**
- Modify: [src/sim/movement/movement_tests.rs](../../src/sim/movement/movement_tests.rs) — append to the test module

**Test harness (real, verified):**

The actual movement test harness is `tick_movement_with_grid` at [src/sim/movement/mod.rs:218](../../src/sim/movement/mod.rs#L218). Existing tests in [movement_tests.rs:71-99](../../src/sim/movement/movement_tests.rs#L71-L99) construct entities via `GameEntity::test_default(id, type_ref, owner, rx, ry)` and call the simpler `tick_movement(entities, ms, interner)` wrapper which passes `path_grid: None`. For bridge tests we need to use the `_with_grid` variant directly.

`tick_movement_with_grid` signature:
```rust
pub fn tick_movement_with_grid(
    entities: &mut EntityStore,
    path_grid: Option<&PathGrid>,
    terrain_costs: &BTreeMap<SpeedType, TerrainCostGrid>,
    alliances: &HouseAllianceMap,
    occupancy: &mut OccupancyGrid,
    rng: &mut SimRng,
    tick_ms: u32,
    sim_tick: u64,
    interner: &mut StringInterner,
) -> MovementTickStats
```

Important: `GameEntity::test_default` populates `locomotor: None` ([game_entity.rs:279](../../src/sim/game_entity.rs#L279)). For drive-locomotor entities, the test must explicitly assign a `LocomotorState` (use the same hand-listed pattern as Task 7's `make_loco` — see [droppod_movement.rs:205-234](../../src/sim/movement/droppod_movement.rs#L205-L234) for the canonical example).

**Step 1: Add a shared test helper** at the top of `movement_tests.rs` (near the existing imports) — `tick_with_grid_helper` to avoid repeating the 9-arg call. Find the existing imports block and add:

```rust
use std::collections::BTreeMap;
use crate::sim::movement::tick_movement_with_grid;
use crate::sim::pathfinding::PathGrid;
use crate::sim::movement::locomotor::{AirMovePhase, GroundMovePhase, LocomotorState};
use crate::rules::locomotor_type::{LocomotorKind, MovementZone, SpeedType};
use crate::map::houses::HouseAllianceMap;
use crate::util::fixed_math::{SIM_ONE, SIM_ZERO};

fn make_drive_loco(layer: MovementLayer) -> LocomotorState {
    LocomotorState {
        kind: LocomotorKind::Drive,
        layer,
        phase: GroundMovePhase::Idle,
        air_phase: AirMovePhase::Landed,
        speed_multiplier: SIM_ONE,
        speed_fraction: SIM_ONE,
        fly_current_speed: SIM_ZERO,
        altitude: SIM_ZERO,
        target_altitude: SIM_ZERO,
        climb_rate: SIM_ZERO,
        jumpjet_speed: SIM_ZERO,
        jumpjet_wobbles: 0.0,
        jumpjet_accel: SIM_ZERO,
        jumpjet_current_speed: SIM_ZERO,
        jumpjet_deviation: 0,
        jumpjet_crash_speed: SIM_ZERO,
        jumpjet_turn_rate: 4,
        balloon_hover: false,
        hover_attack: false,
        speed_type: SpeedType::Track,
        movement_zone: MovementZone::Normal,
        rot: 0,
        override_state: None,
        air_progress: SIM_ZERO,
        infantry_wobble_phase: 0.0,
        subcell_dest: None,
    }
}

fn tick_bridge(
    entities: &mut EntityStore,
    grid: &PathGrid,
    occupancy: &mut OccupancyGrid,
    rng: &mut SimRng,
    interner: &mut crate::sim::intern::StringInterner,
    ms: u32,
) {
    let costs: BTreeMap<SpeedType, crate::sim::pathfinding::TerrainCostGrid> = BTreeMap::new();
    let alliances = HouseAllianceMap::default();
    let _ = tick_movement_with_grid(
        entities, Some(grid), &costs, &alliances, occupancy, rng, ms, 0, interner,
    );
}
```

**Step 2: Add the three integration tests** at the end of the test module:

```rust
#[test]
fn on_bridge_fires_at_ramp_to_body_only() {
    // Layout: 10x10 grid. (1,1) is a ramp/bridgehead at raw h=4 (bridge_walkable, transition=true).
    // (2,1) is a body cell at raw h=0 (bridge_walkable, no transition). Effective deck = 4.
    let mut grid = PathGrid::new(10, 10);
    grid.set_cell_for_test(1, 1, 4, true, true);
    grid.set_cell_for_test(2, 1, 0, true, false);

    let mut entities = EntityStore::new();
    let mut e = GameEntity::test_default(1, "HTNK", "Americans", 1, 1);
    e.position.z = 4;                            // unit visually at deck level
    e.on_bridge = false;                         // not yet on body deck per predicate
    e.locomotor = Some(make_drive_loco(MovementLayer::Bridge));
    e.movement_target = Some(MovementTarget {
        path: vec![(1, 1), (2, 1)],
        path_layers: vec![MovementLayer::Bridge, MovementLayer::Bridge],
        next_index: 1,
        speed: SimFixed::from_num(2560),
        move_dir_x: SimFixed::from_num(256),
        move_dir_y: SIM_ZERO,
        move_dir_len: SimFixed::from_num(256),
        ..Default::default()
    });
    entities.insert(e);

    let mut occupancy = OccupancyGrid::new();
    occupancy.add(1, 1, 1, MovementLayer::Bridge, None);
    let mut rng = SimRng::new(0);
    let mut interner = test_interner();

    // Pre-tick assertion: on_bridge is false on the ramp.
    assert!(!entities.get(1).unwrap().on_bridge, "pre-tick: on_bridge must be false on ramp");

    // Advance one large tick — unit reaches (2,1) and the predicate fires Enter.
    tick_bridge(&mut entities, &grid, &mut occupancy, &mut rng, &mut interner, 1000);

    let entity = entities.get(1).expect("entity exists");
    assert_eq!(entity.position.rx, 2);
    assert_eq!(entity.position.ry, 1);
    assert!(entity.on_bridge, "on_bridge must fire on Ramp→Body transition");
    assert_eq!(
        entity.bridge_occupancy.as_ref().expect("BridgeOccupancy set on Enter").deck_level,
        4
    );
}

#[test]
fn on_bridge_clears_at_ramp_to_ground_only() {
    // Layout: body at (1,1) raw h=0 bridge_walkable; ramp at (2,1) raw h=4 bridge_walkable+transition;
    // ground at (3,1) raw h=4 no bridge_walkable.
    // Path: (1,1) → (2,1) → (3,1). on_bridge must be true through the ramp tick and clear on Ramp→Ground.
    let mut grid = PathGrid::new(10, 10);
    grid.set_cell_for_test(1, 1, 0, true, false);  // body
    grid.set_cell_for_test(2, 1, 4, true, true);   // ramp
    grid.set_cell_for_test(3, 1, 4, false, false); // ground at h=4 (off the bridge)

    let mut entities = EntityStore::new();
    let mut e = GameEntity::test_default(1, "HTNK", "Americans", 1, 1);
    e.position.z = 4;                       // visually at deck
    e.on_bridge = true;                     // unit IS on the deck (predicate already fired Enter previously)
    e.bridge_occupancy = Some(BridgeOccupancy { deck_level: 4 });
    e.locomotor = Some(make_drive_loco(MovementLayer::Bridge));
    e.movement_target = Some(MovementTarget {
        path: vec![(1, 1), (2, 1), (3, 1)],
        // Body → Ramp goes on Ground layer per is_at_bridge_level (parent at deck, neighbor at h=4
        // ground_level → abs(4 - 4) = 0 < 2 → not at bridge level). Ramp → Ground stays Ground.
        path_layers: vec![MovementLayer::Bridge, MovementLayer::Ground, MovementLayer::Ground],
        next_index: 1,
        speed: SimFixed::from_num(2560),
        move_dir_x: SimFixed::from_num(256),
        move_dir_y: SIM_ZERO,
        move_dir_len: SimFixed::from_num(256),
        ..Default::default()
    });
    entities.insert(e);

    let mut occupancy = OccupancyGrid::new();
    occupancy.add(1, 1, 1, MovementLayer::Bridge, None);
    let mut rng = SimRng::new(0);
    let mut interner = test_interner();

    // Tick 1: body → ramp. on_bridge must STAY true (predicate NoChange).
    tick_bridge(&mut entities, &grid, &mut occupancy, &mut rng, &mut interner, 500);
    let entity = entities.get(1).expect("entity exists");
    assert_eq!((entity.position.rx, entity.position.ry), (2, 1), "after tick 1: at ramp");
    assert!(entity.on_bridge, "after tick 1 (on ramp): on_bridge must stay true");

    // Tick 2: ramp → ground. on_bridge must CLEAR (predicate Exit).
    tick_bridge(&mut entities, &grid, &mut occupancy, &mut rng, &mut interner, 500);
    let entity = entities.get(1).expect("entity exists");
    assert_eq!((entity.position.rx, entity.position.ry), (3, 1), "after tick 2: at ground");
    assert!(!entity.on_bridge, "after Ramp→Ground: on_bridge must clear");
    assert!(entity.bridge_occupancy.is_none(), "after Exit: BridgeOccupancy must be None");
}

#[test]
fn no_bridge_lookahead_pre_claim() {
    // Regression: ensure the deleted apply_bridge_lookahead_if_needed has not been
    // reintroduced via another path. BridgeOccupancy must NOT be set before the unit
    // physically crosses onto a body cell.
    // Layout: ground at (1,1) h=4, ramp at (2,1) raw h=4 bridge_walkable+transition,
    // body at (3,1) raw h=0 bridge_walkable. Unit starts on ground; path approaches the bridge.
    let mut grid = PathGrid::new(10, 10);
    grid.set_cell_for_test(1, 1, 4, false, false);  // ground at elevation
    grid.set_cell_for_test(2, 1, 4, true, true);    // ramp
    grid.set_cell_for_test(3, 1, 0, true, false);   // body

    let mut entities = EntityStore::new();
    let mut e = GameEntity::test_default(1, "HTNK", "Americans", 1, 1);
    e.position.z = 4;
    e.on_bridge = false;
    e.bridge_occupancy = None;
    e.locomotor = Some(make_drive_loco(MovementLayer::Ground));
    e.movement_target = Some(MovementTarget {
        path: vec![(1, 1), (2, 1), (3, 1)],
        path_layers: vec![MovementLayer::Ground, MovementLayer::Bridge, MovementLayer::Bridge],
        next_index: 1,
        speed: SimFixed::from_num(2560),
        move_dir_x: SimFixed::from_num(256),
        move_dir_y: SIM_ZERO,
        move_dir_len: SimFixed::from_num(256),
        ..Default::default()
    });
    entities.insert(e);

    let mut occupancy = OccupancyGrid::new();
    occupancy.add(1, 1, 1, MovementLayer::Ground, None);
    let mut rng = SimRng::new(0);
    let mut interner = test_interner();

    // Pre-tick: BridgeOccupancy must be None (we set it so).
    assert!(entities.get(1).unwrap().bridge_occupancy.is_none(), "pre-tick: no pre-claim");

    // Tick 1: ground → ramp. Predicate NoChange (entry needs src_h - 4 == dst_h; here it's
    // 4 - 4 == 0 but ramp's ground_level is 4, not 0 → entry doesn't fire). BridgeOccupancy
    // must STILL be None.
    tick_bridge(&mut entities, &grid, &mut occupancy, &mut rng, &mut interner, 500);
    let entity = entities.get(1).expect("entity exists");
    assert_eq!((entity.position.rx, entity.position.ry), (2, 1), "after tick 1: at ramp");
    assert!(
        entity.bridge_occupancy.is_none(),
        "regression: BridgeOccupancy must NOT be pre-claimed on the ramp"
    );

    // Tick 2: ramp → body. NOW the predicate fires Enter. BridgeOccupancy gets Set.
    tick_bridge(&mut entities, &grid, &mut occupancy, &mut rng, &mut interner, 500);
    let entity = entities.get(1).expect("entity exists");
    assert_eq!((entity.position.rx, entity.position.ry), (3, 1), "after tick 2: on body");
    assert!(entity.on_bridge, "after Ramp→Body: on_bridge must be true");
    assert_eq!(
        entity.bridge_occupancy.as_ref().expect("set on Enter").deck_level,
        4
    );
}
```

**Step 3: If `HouseAllianceMap::default()` doesn't exist**, look at how existing tests construct an empty alliance map (likely `HouseAllianceMap::new()` or similar) and adjust. Similarly for `OccupancyGrid::new()` if the constructor name differs — adapt to the actual repo API as discovered.

**Step 4: Run tests**
Run: `cargo test -p ra2-rust-game --lib movement_tests -- --nocapture 2>&1 | tail -40`
Expected: the 3 new tests PASS plus all existing movement tests PASS. If a 3rd test fails because path_layers[1]=Bridge is wrong for a ramp coming off (gets reassigned by A* to Ground), adjust the test's `path_layers` to match what the production code at line 467 reads from `target.layer_at(next_index)`.

**Step 5: Commit**
```
sim/movement: 3 integration tests for on_bridge timing at ramp boundaries

Pins: predicate fires at Ramp→Body exactly, clears at Ramp→Ground exactly,
no anticipatory BridgeOccupancy pre-claim. Uses tick_movement_with_grid +
hand-built LocomotorState (matches droppod_movement::tests pattern).
```

---

### Task 18: Update TODO(RE) comments

**Why:** Strip stale TODO(RE) notes that referenced the heuristic and the lookahead. The cell_entry.rs TODO about bridge legality is also resolved (now driven via path_layers).

**Files:**
- Modify: [src/sim/movement/movement_bridge.rs](../../src/sim/movement/movement_bridge.rs) (module-level doc already replaced in Task 12)
- Modify: [src/sim/pathfinding/cell_entry.rs](../../src/sim/pathfinding/cell_entry.rs) — lines 12-14

**Step 1: Read cell_entry.rs lines 8-15** for context:

```rust
//! Two-phase design for borrow checker compatibility:
//! - Phase 1 (`check_terrain`): terrain + occupancy presence, no EntityStore needed
//! - Phase 2 (`classify_occupied_cell`): blocker friendship/crush, needs &EntityStore
//!
//! TODO(RE): The stock search-time legality/cost predicate is richer than this runtime
//! movement-side classification. Bridge legality, more terrain/object cases, and the
//! exact cost classes still need to be pulled in from the RE corpus.
```

**Step 2: Replace lines 12-14 with:**

```rust
//! Bridge legality is now driven by A*'s `path_layers` (set per-step by `astar_search`
//! with verified-against-gamemd Ground→Bridge gates), which approximates the post-switch
//! output of gamemd's two-pass `Can_Enter_Cell`. See
//! docs/plans/2026-05-11-bridge-locomotor-layer-correctness-design.md §"Known Parity Boundary".
//!
//! TODO(RE): Cost-class refinements (search-time entity-block costs vs runtime bump) and
//! some terrain edge cases still pending. Tracked separately from G2/G6.
```

**Step 3: Verify compile**
Run: `cargo check -p ra2-rust-game 2>&1 | tail -5`
Expected: clean.

**Step 4: Commit**
```
sim: refresh TODO(RE) comments for bridge-legality and cell_entry
```

---

### Task 19: Full regression run

**Why:** Confirm no test breaks elsewhere. The A* changes may have shifted some path shapes; this catches it.

**Files:** none modified

**Step 1: Run full test suite**
Run: `cargo test -p ra2-rust-game --lib -- --nocapture 2>&1 | tail -60`
Expected: All tests PASS. If any test fails:
- If failure is in a test that used the old heuristic timing (e.g., assumes on_bridge fires at a specific tick that's now off by one), update the test's tick expectation to match the new predicate timing.
- If failure is in an A* test that used `(2..=4)` height-diff routing, update the test setup to use diff exactly 4 with bridgehead.
- DO NOT silence failures with `#[ignore]` — fix or document each one.

**Step 2: Run clippy**
Run: `cargo clippy -p ra2-rust-game -- -D warnings 2>&1 | tail -40`
Expected: no new warnings introduced by this work.

**Step 3: Run `cargo fmt --check`**
Run: `cargo fmt --check 2>&1 | tail -10`
Expected: no diff. If there is, run `cargo fmt` and commit the formatting.

**Step 4: Commit (if Step 3 produced a formatting change)**
```
style: cargo fmt after bridge-locomotor changes
```

If Step 3 was clean, no commit needed for this task.

---

### Task 20: Manual gamemd.exe parity verification

**Why:** The whole point. Final check that the implementation produces output indistinguishable from gamemd.exe on a bridge map.

**Files:** none

**Step 1: Pick a stock YR map with a high bridge** — e.g., "Country Swing" or "Heck Freezes Over" (both have prominent bridges). The exact map doesn't matter as long as it has at least one high bridge with ramps on both ends.

**Step 2: Play a quick skirmish in gamemd.exe** (the original). Move a Rhino or Grizzly across the bridge. Watch carefully for:
- Z-pops at the ramp boundary (sudden vertical jumps)
- Layer flicker (unit briefly disappearing or appearing in wrong rendering order)
- Repath stutters (unit pausing, repathing, then continuing — should NOT happen on a clean bridge crossing)
- Sound cue timing (engine note changes if any — should be smooth)

**Step 3: Reproduce the same skirmish in this engine** (build and run with `cargo run --release`). Do the same crossing. Compare frame-by-frame if possible (screen capture or in-game observation).

**Step 4: Specifically verify on_bridge timing.** Add a debug print or check via in-game inspector (whatever's available):
- `on_bridge` should be `false` while the unit is on the ramp going up.
- `on_bridge` should flip to `true` the tick the unit enters the first body cell (Ramp→Body transition).
- `on_bridge` should stay `true` while the unit is on body cells AND on the ramp coming off.
- `on_bridge` should flip to `false` the tick the unit enters the ground cell after the far ramp (Ramp→Ground transition).

**Step 5: If observable behavior matches gamemd**, this design is done. Note success in the commit message of the final task and check off "Task 20: PASS" mentally.

**Step 6: If observable behavior diverges**, file the divergence as a follow-up issue:
- What specifically diverges (Z-pop / layer flicker / wrong on_bridge timing / repath stutter)?
- At which exact moment in the crossing?
- Is it visible to the player at single-skirmish speed, or only in slow-motion / replay debugging?
- Per the parity bar: divergences visible at normal speed are bugs to fix; divergences only visible in slow-motion are noted but accepted.

**No commit** for this task — it's a verification step. Update the AUDIT_LOG.md with a brief verification entry if the design implementation closes the gap.

---

## Sources & References

- **Design doc:** [docs/plans/2026-05-11-bridge-locomotor-layer-correctness-design.md](2026-05-11-bridge-locomotor-layer-correctness-design.md)
- **Ghidra reports:**
  - [ra2-rust-game-docs/BRIDGE_SYSTEM.md](../../../ra2-rust-game-docs/BRIDGE_SYSTEM.md) — §"Bridge Ramp Detection (Drive Locomotion)", §"CheckBridgeTraversal Complete Logic", §"A* Pathfinding — Dual-Layer Bridge Support", §"RecalcAttributes Bridge Correction"
  - [ra2-rust-game-docs/UNIT_CAN_ENTER_CELL_GHIDRA_REPORT.md](../../../ra2-rust-game-docs/UNIT_CAN_ENTER_CELL_GHIDRA_REPORT.md) §"Phase 6" (two-pass occupancy switch)
  - [ra2-rust-game-docs/AUDIT_LOG.md](../../../ra2-rust-game-docs/AUDIT_LOG.md) entries dated 2026-05-11 (scoped audit + 3-piece inference check + 2-piece follow-up)
- **gamemd.exe addresses** (kept here, NOT in Rust code comments per project policy):
  - `0x4B0F20` — `DriveLocomotionClass::Process_Drive_Track` (on_bridge predicate entry function)
  - `0x4B1812` — predicate entry asm site: `SUB EAX,4; CMP ECX,EAX`
  - `0x4B1830` / `0x4B184A` — predicate exit asm sites
  - `0x4AF4A0` — `DriveLocomotionClass::ComputeBridgeZOffset`: `g_BridgeZOffset_Drive = ftol(g_DriveHeightStep * 4)`
  - `0x4AFD40` — `DriveLocomotionClass::Set_Destination`; Z bump at 0x4AFDD2 → 0x4AFDDD → 0x4AFDE8
  - `0x429A90` — `PathfinderClass::AStar_main_loop` (dual closed lists; `is_at_bridge_level` inline at 0x429E54)
  - `0x4D9C60` — `CheckBridgeTraversal` (height-diff rules)
  - `0x47E040` / `0x47E470` — `SetBridgeDirection_NESW` / `_NWSE` (byte-identical; informational only)
  - `0x47D2B0` — `CellClass::RecalcAttributes`; +0x11B write at 0x47D94E
- **INI keys:** none directly applicable. Bridge mechanics are runtime engine behavior, not INI-driven.
- **Related code:**
  - [src/sim/movement/movement_bridge.rs](../../src/sim/movement/movement_bridge.rs) — primary target
  - [src/sim/movement/movement_step.rs](../../src/sim/movement/movement_step.rs) — main cell-crossing caller
  - [src/sim/movement/movement_tick.rs](../../src/sim/movement/movement_tick.rs) — drive-track-jump caller + render-state apply sites
  - [src/sim/pathfinding/core.rs](../../src/sim/pathfinding/core.rs) — A* tightening (Tasks 14, 15)
  - [src/sim/pathfinding/cell_entry.rs](../../src/sim/pathfinding/cell_entry.rs) — TODO comment refresh (Task 18)
  - [src/sim/movement/bump_crush.rs](../../src/sim/movement/bump_crush.rs) — pattern reference for pure-fn helpers
  - [src/sim/components.rs:200](../../src/sim/components.rs#L200) — `MovementTarget.path_layers` source of truth
