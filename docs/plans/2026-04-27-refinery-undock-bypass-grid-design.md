# Refinery Undock — bypass_grid + A* Start Relaxation — Design

## Goal

Eliminate the harvester head-butt-after-unload bug by replicating gamemd's `BuildingClass::UndockUnit` mechanism: the harvester drives from the dock pad to a fixed point inside the refinery's south-edge foundation cell, then `Mission_Harvest` State 0 takes over and pathfinds out to ore from that blocked starting cell.

## Architecture Context

### The bug

After unloading, [phase_exit_pad](../../src/sim/miner/miner_dock_sequence.rs#L422) calls [issue_direct_move(exit_cell)](../../src/sim/movement/movement_commands.rs#L96). For a 4×3 GAREFN at `(rx, ry)`, [refinery_exit_cell](../../src/sim/miner/miner_dock_sequence.rs#L98-L111) returns `(rx+1, ry+1)` — INSIDE the foundation `(rx..=rx+3, ry..=ry+2)`.

[movement_step.rs:446](../../src/sim/movement/movement_step.rs#L446) checks `path_grid.is_walkable(nx, ny)` regardless of `target.ignore_terrain_cost`. The foundation cells are blocked by [block_building_footprint](../../src/sim/pathfinding/core.rs#L1065). The harvester tries to advance, fails, snaps back, repeats indefinitely — never reaches the `at_exit` arrival check that transitions state to `SearchOre`.

### gamemd mechanism (verified via Ghidra audit, see audit transcript in conversation)

`BuildingClass::UndockUnit` @ 0x004593A0 calls `loco->vtable+0x70(0x47, X-128, Y+128, Z)`, which dispatches to `DriveLocomotionClass::Force_Track` @ 0x004B0C40. Force_Track:
- Stores the first arg (`0x47`) at locomotor field `+0x54` (purpose unverified — not used as a screen-facing).
- Clears `track_index` (+0x58) to 0.
- Sets `head_to` and `destination` to the exit position.
- Sets `is_on_track` = 1 and `current_speed` = 1.0.
- Calls `Apply_Track_Delta` (probably benign occupancy update).

Exit position for the call: `(coord.X - 128, coord.Y + 128, coord.Z)` where `coord` comes from `BuildingClass::GetCoords` @ 0x00447ac0 (verified: vtable+0x48 → 0x447ac0). For a building with foundation `w×h`:
- `coord.X = origin.X + (w-1)*128`
- `coord.Y = origin.Y + (h-1)*128`

For a 4×3 GAREFN at `(rx, ry)` with `origin = cell-center (rx*256+128, ry*256+128)` (verified via `BuildingClass::GetRenderCoords` semantics):
- `exit_world.X = rx*256 + 384` → cell `rx+1` at sub-cell 128 (cell-center)
- `exit_world.Y = ry*256 + 512` → cell `ry+2` at sub-cell 0 (north boundary)
- Exit cell: **(rx+1, ry+2)** — inside the foundation, on its south edge.

The drive uses `is_on_track` semantics that bypass `Can_Enter_Cell` for the duration. After arrival, Mission stays at `Harvest=10` with State `0`. Per-tick:
1. `Mission_Harvest` State 0 calls `Search_For_Tiberium_And_Move` (TiberiumLongScan radius 48).
2. Found ore → `Set_Destination(ore_cell)` → `FootClass::Find_Path` → `AStar_pathfind_search` → `AStar_main_loop` @ 0x429a90.
3. Per [PATHFINDING_ASTAR_GHIDRA_REPORT.md §4.1](../../../ra2-rust-game-docs/PATHFINDING_ASTAR_GHIDRA_REPORT.md), the start node is created and seeded into the open set **without a passability check on the start cell**. Only neighbor expansion calls `Can_Enter_Cell` (vtable+0x1AC).
4. From `(rx+1, ry+2)`, three of eight neighbors are south-of-foundation walkable cells. A* finds the path out.

### Doc errors discovered

- `HARVESTER_DOCK_UNLOAD.md §4` glosses the call as "Head_To with facing=0x47 ≈ ESE" pushing the harvester "one cell southeast of the building center". Both glosses are wrong: Force_Track does not write `0x47` to facing, and the iso `(-128, +128)` offset points southwest (not southeast). The actual end position is inside the foundation, not outside.
- `WAR_MINER_LOCOMOTION_INTEGRATION_GHIDRA_REPORT.md §6` claims the coord-getter is `Get_Coord_Adjusted`. It's `BuildingClass::GetCoords` (vtable+0x48 → 0x447ac0).
- These doc errors will be addressed separately via `/verify-doc` follow-ups; not in scope for this fix.

### Our Rust pathfinder's start-cell rejection

[astar_search](../../src/sim/pathfinding/core.rs#L262-L305) explicitly rejects blocked starts at line 292:

```rust
if !start_passable {
    let alt_layer = ...;
    let alt_passable = grid.is_walkable_on_layer(start.0, start.1, alt_layer);
    if !alt_passable {
        return None;
    }
    return astar_search(grid, start, alt_layer, goal, options);
}
```

This is a Rust-side addition with no equivalent in gamemd. [test_find_path_blocked_start](../../src/sim/pathfinding/core_tests.rs#L160) asserts the rejection. Removing it would let A* find paths from inside-foundation cells the way gamemd does.

## Impact Analysis

**Touches:**

- [src/sim/components.rs](../../src/sim/components.rs) — add `bypass_grid: bool` field to `MovementTarget` (parallel to existing `ignore_terrain_cost`).
- [src/sim/movement/movement_step.rs:446](../../src/sim/movement/movement_step.rs#L446) — gate `path_grid.is_walkable` check on `target.bypass_grid`.
- [src/sim/movement/movement_tick.rs:221](../../src/sim/movement/movement_tick.rs#L221) — defensive: reset `bypass_grid: false` in segment-replan rebuild block (matches existing `ignore_terrain_cost: false`).
- [src/sim/pathfinding/core.rs:292-305](../../src/sim/pathfinding/core.rs#L292-L305) — remove start-passable rejection. A* expands neighbors normally regardless of start-cell passability.
- [src/sim/pathfinding/core_tests.rs:160](../../src/sim/pathfinding/core_tests.rs#L160) — flip `test_find_path_blocked_start` assertion: a blocked-start path should now succeed when at least one neighbor is walkable.
- [src/sim/miner/miner_dock_sequence.rs](../../src/sim/miner/miner_dock_sequence.rs) — fix `refinery_exit_cell` Y formula (off-by-one), update `phase_exit_pad` to set `bypass_grid = true` after `issue_direct_move`, drop `EXIT_FACING` constant and the `entity.facing_target = Some(EXIT_FACING)` line.
- [src/sim/miner/miner_system.rs](../../src/sim/miner/miner_system.rs) — strip throwaway `DIAG[...]` log lines (heartbeat, search_ore variants, move_to_ore variants).
- [src/sim/miner/miner_dock_sequence.rs](../../src/sim/miner/miner_dock_sequence.rs) — also strip `DIAG[exit_pad_arrival]` log line.

**Doesn't touch:**

- Other dock phases (`Approach`, `WaitForDock`, `RotateToPad`, `EnterPad`, `TurnOnPad`, `Unloading`) — out of scope.
- Slave miner system — has its own state machine; not affected.
- Other A* callers — they all keep working; the relaxation is a strict superset of current behavior (now finds paths in cases that previously returned `None`; never returns a worse path).

**Determinism:** the `bypass_grid` flag is a deterministic boolean. Removing the start-cell check does not introduce any non-determinism; A* expansion order is unchanged for walkable starts. `MovementTarget` is `Clone` + `serde::Serialize` + `serde::Deserialize` — adding a `bool` field with a default keeps backward compatibility (will deserialize as `false`).

**Tick ordering:** unchanged. The fix operates inside `phase_exit_pad`, which runs at the existing `tick_miners` slot.

**Snapshot serialization:** the new `bypass_grid` field needs a `#[serde(default)]` annotation so older snapshots load cleanly with the field defaulting to `false`. Same pattern as `ignore_terrain_cost` if it has one.

**Risk areas:**

- **A* start relaxation has the broadest blast radius.** Every A* caller is affected. Mitigation: the change is a strict correctness improvement (closer to gamemd, never returns a worse path). Worst case: a unit pushed into an impassable cell by a bug now finds a path out instead of sitting stuck — better behavior, not worse. The existing `test_find_path_blocked_start` assertion captures this design choice: flipping it from "should be None" to "should find a path through walkable neighbors" makes the new contract explicit.
- **`bypass_grid` MUST NOT bypass occupancy checks.** Other-mover collisions are handled by separate code paths in `movement_step` that don't read this flag — verified at design time, must be re-verified during implementation. Bypassing only `path_grid.is_walkable` (terrain + footprint) is the intended scope.
- **Tests on the `dev` branch.** The 5 most recent miner tests don't touch `phase_exit_pad`. The pre-existing `exit_pad_clears_ore_targets_on_arrival` test exercises the arrival-detection branch, which is preserved. The `exit_pad_blocks_transition_during_teleport` test exercises the teleport-gate, also preserved.

## Chosen Approach

True 1:1 parity with gamemd's `UndockUnit` mechanism. Three coordinated changes:

1. **`bypass_grid` flag on `MovementTarget`** — lets `phase_exit_pad`'s `issue_direct_move` step through blocked foundation cells during the brief drive. Mirrors gamemd's `is_on_track` bypass of `Can_Enter_Cell`.
2. **A* start-cell relaxation** — lets `Mission_Harvest`'s post-undock `Set_Destination(ore)` find a path out from a blocked start cell. Matches gamemd's `AStar_main_loop` which has no start-cell passability check.
3. **`refinery_exit_cell` formula fix** — exit cell becomes `(rx+1, ry+2)` for 4×3 GAREFN (south-edge inside foundation), matching gamemd's literal endpoint.

Plus three cleanup items:
- Drop `EXIT_FACING = 0x47` constant and the `facing_target = Some(EXIT_FACING)` write in `phase_exit_pad`. Per the audit, `0x47` is not a screen-facing — it's stored at locomotor field `+0x54` whose purpose is unverified. The harvester's facing during/after undock should derive naturally from the locomotor's source-to-dest computation.
- Strip `DIAG[...]` log lines added during diagnosis.
- Leave the workaround comments about ore-tile traversal at [miner_system.rs:436-441](../../src/sim/miner/miner_system.rs#L436-L441) (the `// Adjacent to ore? The passability matrix ...` block) and [miner_system.rs:467-470](../../src/sim/miner/miner_system.rs#L467-L470) (the `// After issuing the A* move, mark it as ignore_terrain_cost ...` block) alone — they're independent of this fix. They handle ore-tile traversal via the existing `ignore_terrain_cost` flag, NOT foundation traversal via the new `bypass_grid` flag. The DIAG insertions had pushed prior references to "lines 340-343" out of date; those line numbers no longer point to the workaround.

## Design

### Components

#### `MovementTarget::bypass_grid`

```rust
// In src/sim/components.rs MovementTarget
/// When true, the movement tick skips PathGrid walkability checks for cell entry.
/// Used by dock-sequence direct moves where the harvester must traverse the
/// refinery foundation footprint (cells marked blocked by block_building_footprint).
/// Does NOT bypass entity occupancy checks — other movers still collide.
#[serde(default)]
pub bypass_grid: bool,
```

Default: `false`. Backwards-compatible deserialization via `#[serde(default)]`.

#### `movement_step.rs:446` gate

```rust
// Before:
path_grid.map_or(true, |grid| grid.is_walkable(nx, ny))

// After:
target.bypass_grid || path_grid.map_or(true, |grid| grid.is_walkable(nx, ny))
```

Single-line change. The `terrain_ok` calculation below (line 448-450) is independent and stays as-is — it already short-circuits on `target.ignore_terrain_cost`.

#### `astar_search` start relaxation

Replace lines 292-305 of `pathfinding/core.rs`:

```rust
// Before:
if !start_passable {
    let alt_layer = if start_layer == MovementLayer::Bridge {
        MovementLayer::Ground
    } else {
        MovementLayer::Bridge
    };
    let alt_passable = grid.is_walkable_on_layer(start.0, start.1, alt_layer);
    if !alt_passable {
        return None;
    }
    return astar_search(grid, start, alt_layer, goal, options);
}

// After:
// Start cell may be blocked (e.g. unit standing inside a building footprint after undock).
// Matches gamemd's AStar_main_loop @ 0x429a90: start node is seeded into the open set
// without a passability check; only neighbor expansion calls Can_Enter_Cell.
// If all 8 neighbors are also blocked, A* will exhaust its open set and return None
// naturally — same end result as the old check, but blocked cells with walkable
// neighbors now produce valid paths.
```

The alt-layer fallback was specific to the old start-rejection code path; it's no longer needed because the relaxed A* will naturally try walkable neighbors on the same layer.

#### `refinery_exit_cell` formula

```rust
// Before (off by one in Y):
let exit_x = (rx as i32 * 256 + (width as i32 - 2) * 128) / 256;
let exit_y = (ry as i32 * 256 + height as i32 * 128) / 256;

// After (matches gamemd's coord+(-128,+128) where coord = origin+(w-1,h-1)*128):
let exit_x = (rx as i32 * 256 + (width as i32 - 2) * 128) / 256;
let exit_y = (ry as i32 * 256 + height as i32 * 128 + 128) / 256;
```

For 4×3 GAREFN at (rx, ry):
- `exit_x = (rx*256 + 256) / 256 = rx + 1` (unchanged)
- `exit_y = (ry*256 + 384 + 128) / 256 = (ry*256 + 512) / 256 = ry + 2` (was ry+1)

Exit cell: `(rx+1, ry+2)` — south edge of the foundation, matching gamemd.

#### `phase_exit_pad` wiring

```rust
// Before:
if !moving && !at_exit {
    movement::issue_direct_move(&mut sim.entities, snap.entity_id, exit, snap.speed);
    if let Some(entity) = sim.entities.get_mut(snap.entity_id) {
        entity.facing_target = Some(EXIT_FACING);
    }
    return;
}

// After:
if !moving && !at_exit {
    movement::issue_direct_move(&mut sim.entities, snap.entity_id, exit, snap.speed);
    if let Some(entity) = sim.entities.get_mut(snap.entity_id) {
        if let Some(ref mut mt) = entity.movement_target {
            mt.bypass_grid = true;
        }
    }
    return;
}
```

The `EXIT_FACING` constant and its single use site are removed. The `facing_target` write is dropped — facing derives naturally from `issue_direct_move`'s direction calculation.

### Data Flow

```
phase_exit_pad
  → issue_direct_move(exit_cell)        // creates 2-cell MovementTarget
  → set entity.movement_target.bypass_grid = true

tick_movement (next tick)
  → movement_step advances harvester through foundation cells
    → at line 446: bypass_grid=true short-circuits the path_grid check
    → terrain_ok unchanged (ignore_terrain_cost still true from issue_direct_move)
  → harvester arrives at exit_cell, MovementTarget cleared

phase_exit_pad (next tick after arrival)
  → detects !moving && at_exit
  → clears reserved_refinery, target_ore_cell, last_harvest_cell
  → transitions state to SearchOre

handle_search_ore (next tick)
  → search_local_ore (long scan radius) finds ore
  → sets target_ore_cell, transitions to MoveToOre

handle_move_to_ore (next tick)
  → issue_move_command(ore_cell)        // A* with relaxed start
    → A* expands neighbors of (rx+1, ry+2)
    → 3 of 8 neighbors are south-of-foundation walkable
    → finds path out, harvester drives to ore
```

### Determinism Considerations

- `bypass_grid` is a deterministic boolean set at command-issue time, read at tick time.
- A* relaxation does not change neighbor expansion order or node-selection logic. Same paths produced for walkable starts; new paths for blocked starts (where `None` was returned before).
- No RNG, no float math, no time-dependent behavior introduced.
- Snapshot serialization: `#[serde(default)]` on the new field means older snapshots deserialize with `bypass_grid = false`, matching the pre-fix behavior for any in-flight saved games.

### Testing Strategy

**New test in `miner_tests.rs`**:

```
test_harvester_undocks_through_foundation_to_outside_ore:
  - Place 4×3 GAREFN at (10, 10)
  - Mark foundation blocked in path_grid (block_building_footprint)
  - Place HARV at pad cell (13, 11) with cargo, state=Dock, dock_phase=ExitPad,
    reserved_refinery = GAREFN id
  - Place ore at (11, 14) — south of foundation, reachable
  - Tick simulation N times (enough for: drive to exit + 2 ticks for state
    transitions + drive toward ore)
  - Assert: harvester position progresses through foundation cells (e.g. (12,11)
    or (11,11)) without snapping back
  - Assert: harvester reaches exit cell (11, 12)
  - Assert: state transitions Dock → SearchOre → MoveToOre
  - Assert: target_ore_cell == Some((11, 14)) after SearchOre
  - Assert: harvester eventually moves out of foundation (position.ry > 12)
```

**New test in `core_tests.rs`** (replaces flipped `test_find_path_blocked_start`):

```
test_find_path_blocked_start_finds_path_through_walkable_neighbors:
  - Create 10×10 PathGrid
  - Block (5,5)
  - find_path((5,5), (8,5)) should return Some(path) with first step adjacent to (5,5)

test_find_path_blocked_start_all_neighbors_blocked_returns_none:
  - Create 10×10 PathGrid
  - Block (5,5) and all 8 neighbors
  - find_path((5,5), (8,5)) should return None (no escape)
```

**Existing tests**:

- All 5 miner tests on `dev` branch — should pass unchanged.
- `exit_pad_clears_ore_targets_on_arrival` — verifies `target_ore_cell = None` and `last_harvest_cell = None` post-arrival. Preserved.
- `exit_pad_blocks_transition_during_teleport` — verifies the teleport-gate. Preserved.
- All A* tests other than the flipped one — should pass unchanged (relaxation is additive).

## Architectural Decisions

**Pattern followed:** the new `bypass_grid` field mirrors the existing `ignore_terrain_cost` pattern in `MovementTarget` — both are bool flags read at movement-tick time to gate specific passability checks. Same shape, same lifecycle, same default.

**Pattern deviated from:** the A* start-cell rejection was a Rust-only invention (no gamemd equivalent). Removing it brings our pathfinder closer to the original engine's design and removes an asymmetry between "can enter" (per-cell check) and "currently in" (was a hard-rejection).

**Tech debt:** none introduced. The change strictly reduces code (deletes the start-rejection branch, removes the `EXIT_FACING` constant, removes the DIAG logs). The `bypass_grid` field is the only addition.

**Follow-ups (not in scope):**
- `/verify-doc` pass on `HARVESTER_DOCK_UNLOAD.md` and `WAR_MINER_LOCOMOTION_INTEGRATION_GHIDRA_REPORT.md` to fix the doc errors found during the audit.
- The medium-severity miner gaps (G3 refinery-destroyed mid-dock, G4 ROT=10 global, G5 purifier formula) — independent and lower priority.

## Alternatives Considered

**A. Pragmatic outward-vector exit (rejected).** Pick `exit_cell = (rx+1, ry+3)` outside the foundation south edge; bypass_grid for the brief drive; no A* changes. Smaller diff (~10 LOC), but observable result drifts from gamemd in two ways: end position one cell further south, and a ~50ms pause at the exit cell that gamemd doesn't have. Rejected because the A* relaxation is a strict correctness improvement and the parity gain is meaningful.

**B. Direction-based "drive impulse" mode (rejected).** New `MovementTarget::DriveImpulse { facing, distance }` variant handled by movement_step without grid checks. More invasive (new movement variant, new state machine branches), doesn't match gamemd's `Force_Track` which is destination-based, not direction-based. Rejected — solves a problem we don't have.

**C. Extend `ignore_terrain_cost` to also bypass path_grid (rejected).** Repurpose the existing flag to skip both terrain costs and footprint blocking. Smallest possible diff (rename + extend semantics), but creates hidden coupling: every existing `ignore_terrain_cost` caller (currently only `issue_direct_move` for ore-cell traversal in `handle_move_to_ore` post-A*) would silently start bypassing footprint checks too. The ore-traversal case actually only needs the terrain bypass, not the footprint bypass — folding them together breaks separation of concerns. Rejected.

**D. Literal facing 0x47 ESE (rejected).** Set `entity.facing_target = 0x47` to "match gamemd". Per the Ghidra audit, `0x47` is not a facing — it's a value stored at locomotor field `+0x54` of unverified purpose. Writing it as a facing is speculative and produces the wrong visual (harvester points ESE while moving SW). Rejected — better to let the locomotor compute facing naturally from movement direction.
