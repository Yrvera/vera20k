# Building Scatter Fix — Design

## Goal

Stop refinery (and any building) from being issued a movement command when a
harvester drives into its foundation cells during the dock sequence.

## Architecture Context

### How the dock drive currently flows

1. The miner state machine ([miner_dock_sequence.rs](src/sim/miner/miner_dock_sequence.rs))
   issues `movement::issue_direct_move` with `bypass_grid=true` to drive the harvester
   into a refinery's foundation footprint (rotate-into-pad, exit-pad).
2. `bypass_grid` was introduced to relax the **path_grid** walkability check so the
   harvester can step into cells marked unwalkable by the foundation. It does NOT
   relax the **occupancy** check.
3. Each cell crossing in `process_cell_crossings` queues a deferred occupancy check.
4. `handle_deferred_occupancy` ([movement_occupancy.rs:117](src/sim/movement/movement_occupancy.rs#L117))
   calls `cell_entry::classify_occupied_cell`, which calls `find_primary_blocker`.
5. The refinery's foundation cells are registered in `OccupancyGrid` with the
   refinery's stable_id, so the building is picked as the primary blocker.
6. `classify_blocker` returns `OccupiedFriendly { blocker_id: refinery_sid }`.
7. The match arm at [movement_occupancy.rs:222](src/sim/movement/movement_occupancy.rs#L222)
   calls `bump_crush::scatter_blocker(refinery_sid, …)`.
8. `scatter_blocker` has no Structure guard. `issue_direct_move` has no Structure guard
   in `can_accept_destination`. The refinery gets a `movement_target` attached and
   walks to a random adjacent cell.

### gamemd parity reference

- `vtable+0x174` Scatter slot exists only for `UnitClass` (0x743A50) and
  `InfantryClass` (0x51D0D0). No `BuildingClass::Scatter`. Source:
  [SCATTER_DISPATCH_SYSTEM_DEEP_DIVE.md](docs/research/SCATTER_DISPATCH_SYSTEM_DEEP_DIVE.md).
- `CellClass::Scatter_Objects` runs each occupant through `FilterToTechno` which
  rejects RTTI 6 (Building) before per-class dispatch.
- Buildings mark cells with bit 0x40 ("Building present"), units mark with bit 0x20
  ("Vehicle/unit"). Source:
  [CELL_OCCUPATION_MARKING_GHIDRA_REPORT.md](docs/research/CELL_OCCUPATION_MARKING_GHIDRA_REPORT.md).
  The "blocked by friendly" check only considers bit 0x20.
- The harvester dock drive is choreographed via radio commands (`radio(0xE)`
  CAN_DOCK → `radio(0x15)` DOCK_NOW → `radio(0x18)` begin dock), not normal
  pathfinding. Source:
  [MISSION_ENTER_REFINERY_DOCK_GHIDRA_REPORT.md](docs/research/MISSION_ENTER_REFINERY_DOCK_GHIDRA_REPORT.md).

Net: gamemd treats buildings as immutable obstacles, never eligible for scatter,
both at class-type level and at dispatch-filter level.

## Impact Analysis

### Files touched

- `src/sim/pathfinding/cell_entry.rs` — add `mover_bypass_grid` param to
  `classify_occupied_cell` and `find_primary_blocker`; extend `find_primary_blocker`
  to take `&EntityStore` and skip Structure occupants when bypass is set.
- `src/sim/movement/movement_occupancy.rs` — read `bypass_grid` from the mover's
  `MovementTarget` and pass into `classify_occupied_cell`.
- `src/sim/movement/movement_tick.rs` — same plumbing wherever it constructs the
  occupancy check call.
- `src/sim/movement/bump_crush.rs` — add `EntityCategory::Structure` early-return
  in `scatter_blocker`, placed before the RNG read.

### What depends on the changed code

- `bypass_grid=true` callers: only the four miner dock-sequence sites
  (rotate-into-pad, exit-pad direct moves). Tight blast radius.
- `scatter_blocker` callers: two sites in `movement_occupancy.rs` (immediate
  scatter on `OccupiedFriendly`, and deferred scatter after blocked_delay
  expires). Both use the same code path.

### Risk areas

- **Determinism (sacred for lockstep).** `scatter_blocker` consumes RNG at
  `rng.next_range_u32(8)`. The Structure guard MUST be placed before that read,
  so RNG consumption order is unchanged for every legitimate (non-Structure)
  scatter case. The only RNG-order change is for Structure blocker_ids — which
  in old buggy code consumed RNG to issue an invalid building move; in new code
  consume nothing. No legitimate replays exist to preserve from the bug regime.
- **State hash:** no new entity fields, no tick-order change. Unaffected.
- **Existing tests:** all current miner/dock tests construct `OccupancyGrid::new()`
  (empty), so they don't hit this path. They remain green.

## Chosen Approach

**Approach A: filter at the source (`find_primary_blocker`).**

When the mover has `bypass_grid=true`, the blocker scan in `find_primary_blocker`
skips entries whose `entity.category == Structure`. If only structures are present,
the function returns `None`, which `classify_occupied_cell` treats as no blocker
(falls through to Phase 1's `Clear` path).

A second-line defense lives in `scatter_blocker`: even if some future caller
produces an `OccupiedFriendly` with a Structure blocker_id, the building still
won't be moved.

### Why this over alternatives

- **Approach B (filter at consumer):** keeps the cell-entry result as
  `OccupiedFriendly` and reinterprets it at the match arm. Says one thing, means
  another — confuses future readers and any debug trace tooling. Rejected.
- **Approach C (separate structure occupancy bit):** the architecturally correct
  long-term model, mirroring gamemd's 0x20 vs 0x40 split. Wide refactor across
  every `OccupancyGrid` consumer. Out of scope for this fix; filed as follow-up.

## Design

### Components

1. `cell_entry.rs::find_primary_blocker`
   - Add params: `mover_bypass_grid: bool`, `entities: &EntityStore`.
   - Iterate `occ.blockers(layer)`. When `mover_bypass_grid` is true, skip ids
     whose `entities.get(id).map(|e| e.category) == Some(EntityCategory::Structure)`.
   - Existing infantry fallback unchanged.

2. `cell_entry.rs::classify_occupied_cell`
   - Add param `mover_bypass_grid: bool`, forward to `find_primary_blocker`.
   - No other behavior change.

3. `movement_occupancy.rs::handle_deferred_occupancy`
   - Before calling `classify_occupied_cell`, read
     `entities.get(entity_id).and_then(|e| e.movement_target.as_ref())
     .map(|mt| mt.bypass_grid).unwrap_or(false)` and pass through.
   - Alternative: extend `MoverSnapshot` with `bypass_grid: bool` and read at
     snapshot construction in `movement_tick.rs`. Use whichever matches existing
     pattern at the call site.

4. `bump_crush.rs::scatter_blocker`
   - At top of function, after the entity-exists check and before `movement_target`
     check (or merged with it):
     ```
     if blocker.category == EntityCategory::Structure {
         return false;
     }
     ```
   - Placed before `rng.next_range_u32(8)` to preserve determinism.

### Interfaces / Contracts

```rust
// cell_entry.rs
pub fn classify_occupied_cell(
    target: (u16, u16),
    target_layer: MovementLayer,
    mover_id: u64,
    mover_zone: MovementZone,
    mover_omni_crusher: bool,
    mover_owner: &str,
    mover_locomotor: LocomotorKind,
    mover_bypass_grid: bool,           // NEW
    occupancy: &OccupancyGrid,
    entities: &EntityStore,
    alliances: &HouseAllianceMap,
    interner: &StringInterner,
) -> CellEntryResult;

fn find_primary_blocker(
    target: (u16, u16),
    layer: MovementLayer,
    mover_id: u64,
    mover_bypass_grid: bool,           // NEW
    occupancy: &OccupancyGrid,
    entities: &EntityStore,            // NEW
) -> Option<u64>;
```

```rust
// bump_crush.rs
pub fn scatter_blocker(
    entities: &mut EntityStore,
    blocker_id: u64,
    path_grid: Option<&PathGrid>,
    occupancy: &OccupancyGrid,
    layer: MovementLayer,
    rng: &mut SimRng,
) -> bool {
    let Some(blocker) = entities.get(blocker_id) else { return false; };
    if blocker.category == EntityCategory::Structure {
        return false;
    }
    if blocker.movement_target.is_some() { return false; }
    // ... existing body unchanged
}
```

### Data Flow

Before:
```
mover hits foundation cell
  → classify_occupied_cell → find_primary_blocker → refinery_sid
  → classify_blocker → OccupiedFriendly { blocker_id: refinery_sid }
  → scatter_blocker(refinery_sid)
  → issue_direct_move(refinery_sid)
  → refinery walks
```

After:
```
mover hits foundation cell
  → classify_occupied_cell(..., mover_bypass_grid=true, ...)
    → find_primary_blocker filters Structure → returns None
    → classify_occupied_cell returns Impassable (no blocker, but cell occupied)
       — or Clear, see Open Questions below
  → mover proceeds through foundation
```

### Open Question

`find_primary_blocker` returning `None` currently leads to `Impassable` (line 187
of cell_entry.rs: "No identifiable blocker (shouldn't happen if Phase 1 said
NeedsBlockerCheck)"). For our case, the cell IS occupied (by a structure) but the
mover should treat it as `Clear`. Two options at implementation time:

- **Option 1:** When `mover_bypass_grid` is true and `find_primary_blocker` returns
  None, return `Clear` instead of `Impassable`.
- **Option 2:** Have Phase 1 (`check_terrain`) skip occupancy presence detection
  for cells whose only occupants are structures, when `mover_bypass_grid` is true.

Option 1 is the smaller change. Pick at implementation time after re-reading
`check_terrain`'s flow.

### Error Handling

- `entities.get(blocker_id)` None → return false / Impassable, matching existing
  fallthrough behavior. No new error path.
- No fallible operations introduced.

### Testing Strategy

1. **`bump_crush.rs` unit test — `scatter_blocker_skips_structure`:**
   - Insert a Structure entity at (5,5).
   - Call `scatter_blocker(structure_id, …)`.
   - Assert returns false, structure has no `movement_target`.

2. **`cell_entry.rs` unit test — `classify_occupied_with_bypass_grid_skips_structure`:**
   - `OccupancyGrid` with a Structure registered at (5,5).
   - Call `classify_occupied_cell(..., mover_bypass_grid=true, ...)`.
   - Assert result is `Clear` (or whichever resolution Open Question lands on).
   - Same call with `mover_bypass_grid=false` → still `OccupiedFriendly` (regression
     check).

3. **`miner_tests.rs` integration test — `harvester_drives_into_refinery_foundation_without_bumping_it`:**
   - Spawn 4×3 refinery at (10,10), record its `position.rx/ry/sub_x/sub_y`.
   - Register all 12 foundation cells in `OccupancyGrid` with the refinery's id.
   - Spawn harvester at queue cell, state=Dock, dock_phase=RotateToPad, cargo full.
   - Tick miner+movement for ~30 ticks.
   - Assert refinery `position` fields all unchanged.
   - Assert refinery `movement_target` is None.
   - Assert harvester progressed (position moved closer to pad OR dock_phase
     advanced past RotateToPad).

4. **Existing tests stay green:** `cargo test miner` and `cargo test movement`
   should pass with no changes — those tests use empty occupancy and don't hit
   the new path.

## Architectural Decisions

- **Pattern followed:** `classify_occupied_cell` already accepts mover-specific
  params (zone, crusher, locomotor). `mover_bypass_grid` joins that group.
- **Pattern deviation:** `find_primary_blocker` previously didn't take
  `&EntityStore` (only occupancy). Adding it is a small expansion but
  `classify_occupied_cell` already has it, so module-level consistency holds.
- **Tech debt:** Approach C (separate structure-occupancy bit, mirroring gamemd's
  bit 0x20 vs 0x40 split) is the proper long-term model. The current fix solves
  the symptom and matches gamemd's observable behavior, but if more `bypass_grid`
  call sites or other "category-aware blocking" needs arise, we'll keep paying
  the param-passing tax. Filed as follow-up.

## Alternatives Considered

- **Approach B — filter at consumer:** reinterpret `OccupiedFriendly` →
  `Clear` at the `handle_deferred_occupancy` match arm. Smaller diff but creates
  a "says one thing, means another" smell — debug traces would still show
  `OccupiedFriendly` even though the mover treats the cell as clear. Rejected
  for clarity.
- **Approach C — separate structure-occupancy bit:** the gamemd-faithful model.
  Wide refactor across every `OccupancyGrid` consumer. Out of scope; filed as
  follow-up.
- **Guard in `can_accept_destination` only:** would block `issue_direct_move` from
  attaching `movement_target` to buildings, but leaves `scatter_blocker` consuming
  RNG for nothing and produces a misleading "scatter succeeded" signal in the
  caller. Rejected — fix at the level where the wrong call originates.
