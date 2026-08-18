# Infantry-From-Barracks Exit Fix Design

## Goal

Stop the Rust port from teleporting barracks-produced infantry to an
`ExitCoord`-derived cell. Spawn them at the building's foundation-center cell
(matching gamemd's `building->GetCoord()` fall-through) and let the existing
pathfinder + rally MoveTo walk them out through the foundation cells.

## Architecture Context

The barracks → infantry spawn pipeline today lives in two files:

- [src/sim/production/production_spawn.rs](src/sim/production/production_spawn.rs)
  - `find_spawn_cell_for_owner` picks the producing factory, then dispatches
    to `find_spawn_cell_near_structure`.
  - `find_spawn_cell_near_structure` calls `preferred_exit_offsets(rules,
    structure_id)`, which — for any factory with `ExitCoord=X,Y,Z` in INI —
    rounds the leptons to a **cell offset** via `lepton_to_cell` and uses
    them as the primary candidate. For YR's barracks
    (`YABRCK: ExitCoord=-64,64,0`), this rounds to `(0, 0)` — i.e. the
    factory's anchor cell. Each candidate then passes through
    `cell_available_for_spawn`, which rejects cells with the `0x40`
    building-occupancy bit.
- [src/sim/production/production_queue.rs](src/sim/production/production_queue.rs)
  - Receives the chosen cell, calls `sim.spawn_object(type, owner, rx, ry,
    facing=64, …)`.
  - If a rally point exists for the owner, issues
    `issue_move_command_with_layered(stable_id, (tx, ty), speed, …)` —
    otherwise the unit just sits at the chosen cell.

Sub-cell + visual-position assignment for the newly-spawned infantry lives
inside `spawn_object`'s infantry block at
[src/sim/world/world_spawn.rs:351-357](src/sim/world/world_spawn.rs#L351-L357):
`sub_cell = allocate_infantry_sub_cell(rx, ry)` (linear scan over
`FUNCTIONAL_SUB_CELLS = [2, 3, 4]`, so an empty cell returns `2 = NE`), then
`position.sub_x/sub_y = subcell_lepton_offset(sub_cell)`.

What gamemd's `BuildingClass::ExitObject_Main` alt path at `0x443F54` does for
infantry (GREEN-verified, RALLY_POINTS_AND_UNIT_SPAWNING.md §6):

1. `Unlimbo` at `building->GetDockCoord()` which, for a default barracks
   (no Weeder/Refinery/Bunker/UnitRepair flags), falls through to
   `FUN_005F6C80` → `building->GetCoord()` = the building's center lepton
   coord at `building+0x9C..0xA4`. `ExitCoord` is **not** read for infantry.
2. Default `SubCell = 2` (NE) from `InfantryClass` constructor — that is what
   gets marked in the cell occupancy bits at spawn.
3. If `RallyTarget != NULL` AND infantry is not amphibious:
   `MoveTo(rally, 1)` + `SetMission(MOVE = 2)`. Pathfinder walks the infantry
   out through the foundation cells (which are passable for infantry — only
   vehicles are hard-blocked by occupancy bit `0x20`).
4. On each walk step, `WalkLoco::FindSubCellDest` calls `PlaceInfantryInCell`
   on the *destination* cell to pick the sub-cell anchor for that cell. This
   is the only place where the random-rotation table at `0x0081CC98` and the
   directional preference table at `0x0081CC84` are consulted.

## Impact Analysis

**Files changed (production code):**
- [src/sim/production/production_spawn.rs](src/sim/production/production_spawn.rs) —
  add `find_infantry_spawn_cell_near_structure`; branch `find_spawn_cell_for_owner`
  by category before reaching `find_spawn_cell_near_structure`.

**Files changed (tests):**
- [src/sim/production/production_tests.rs](src/sim/production/production_tests.rs) —
  the existing infantry-ExitCoord tests at lines 658-732 assert the
  old (wrong) behavior and must be replaced with foundation-center-cell
  assertions. Vehicle ExitCoord parsing tests (`exit_coord_parsed_and_used_for_spawn`
  for `GAWEAP`) stay; vehicle path is out of scope and unchanged.
- [src/sim/production/production_placement_tests.rs](src/sim/production/production_placement_tests.rs) —
  the two `find_spawn_cell_for_owner` calls at lines 848, 879 need review
  for which category they exercise.

**What depends on what we're changing:**
- `find_spawn_cell_for_owner` is called from `production_queue.rs:509`. The
  return shape (`Option<(u16, u16)>`) is unchanged; no consumer changes
  needed.
- `preferred_exit_offsets` is a private helper inside `production_spawn.rs`,
  not consumed by other modules. Safe to leave alone for the vehicle/aircraft
  path.

**Determinism:** the new path is purely arithmetic — `foundation_center_cell =
(foundation_TL.rx + w / 2, foundation_TL.ry + h / 2)`. No RNG, no float, no
iteration order dependency. Lockstep-safe.

**Blast radius:**
- Vehicle/aircraft spawn path: untouched, still uses `preferred_exit_offsets`.
- Naval spawn (require_water=true): untouched (naval factories produce
  vehicles, not infantry).
- Aircraft helipad spawn: untouched (separate path via `find_helipad_for_aircraft`).
- Sub-cell allocation (`allocate_infantry_sub_cell`): untouched. For an
  empty foundation-center cell it returns `2 = NE`, matching gamemd's
  constructor default.

**What might break:**
- Any test that asserts `spawn_object` lands infantry at the ExitCoord-derived
  cell (covered above).
- Visual: infantry now appears *inside* the foundation footprint for one
  tick before walk starts, instead of at the wrong off-foundation cell.
  This matches gamemd — flagging it for visual regression testing rather
  than as a bug.

## Chosen Approach

**Approach B: dedicated `find_infantry_spawn_cell_near_structure` function.**

Split the dispatch at `find_spawn_cell_for_owner`:
- `ObjectCategory::Infantry` → new `find_infantry_spawn_cell_near_structure`
  that returns the foundation-center cell unconditionally (no `ExitCoord`,
  no passability check, no `nearest_walkable_around` fallback — gamemd's
  alt path has no fallback either).
- `Vehicle` / `Aircraft` → existing `find_spawn_cell_near_structure` path.
  Still parity-wrong for ExitCoord semantics, but scope-deferred.

Why over Approach A (per-RTTI branch inside `preferred_exit_offsets`):
infantry and vehicle exits are *different mechanisms* in gamemd (alt-path
Unlimbo-at-center vs `GetDockCellForObject` + barracks-flag-gated lepton
add). Encoding them as separate functions reflects that domain reality and
makes the future vehicle/aircraft fix a parallel addition rather than a
deeper branching tree.

Why over Approach C (intercept at `production_queue.rs`): keeps spawn-cell
selection inside `production_spawn.rs` where it belongs; doesn't grow the
already-large queue tick loop.

## Tiny-Detail Ledger

The parity-relevant tiny details this design must preserve. Each item cites
its source. **Items #10, #11, #12, #13 are flagged as deferred follow-ups
(see end of doc) — they are NOT in this design's implementation scope but
must not be silently dropped.**

| # | Detail | Source | Covered by |
|---|---|---|---|
| 1 | Infantry spawn position = foundation center cell (`(foundation_TL + foundation_size / 2)`), matching gamemd's `building->GetCoord()` → cell containing that lepton | [doc: RALLY_POINTS §6 step 4] [GHIDRA `0x00443C60` alt path `0x443F54`] | `find_infantry_spawn_cell_near_structure` body |
| 2 | `ExitCoord` (`BuildingTypeClass+0xEC8/0xECC/0xED0`) is **never read** for infantry | [doc: RALLY_POINTS §6 "What does NOT happen"] | `find_infantry_spawn_cell_near_structure` does not read `obj.exit_coord` |
| 3 | `GetDockCellForObject` is **never called** for infantry | [doc: RALLY_POINTS §6 same block] | Same — no equivalent helper invoked |
| 4 | `ExitList` (`Type+0xED4`) is **never iterated** for infantry | [doc: RALLY_POINTS §6 same block] | Same |
| 5 | Default SubCell on construction = 2 (NE) | [doc: INFANTRYCLASS_GHIDRA_REPORT.md §2] [GHIDRA `0x00517A50` `param_1[0x1BA]=2`] | Existing `allocate_infantry_sub_cell` returns `FUNCTIONAL_SUB_CELLS[0] = 2` for empty cell. No change needed. |
| 6 | Sub-cell offset table: 0=128/128, 1=64/64, 2=192/64, 3=64/192, 4=192/192 (leptons) | [doc: RALLY_POINTS §7] [INFANTRY_SUBCELL §"Sub-Cell Offset Table"] | Existing `subcell_lepton_offset` in `util/lepton.rs`. No change. |
| 7 | Functional sub-cells = {2, 3, 4} only; 0 and 1 are always "unavailable" | [doc: INFANTRY_SUBCELL §"Occupancy Check"] | Existing `FUNCTIONAL_SUB_CELLS = [2, 3, 4]`. No change. |
| 8 | After Unlimbo, if rally set AND not amphibious → `MoveTo(rally, 1)` + `SetMission(MOVE = 2)` | [doc: RALLY_POINTS §6 step 4 last paragraph] | Existing `production_queue.rs:552-587` rally-MoveTo block. No change. |
| 9 | If rally unset → infantry stays at building-center cell (gamemd issues no move in the alt path) | [doc: RALLY_POINTS §6 step 4 — absence of else branch] | Existing `if let Some((tx, ty)) = rally_point` guard. No change. |
| 10 | Amphibious produced infantry (`Type+0xE0D`) from non-amphibious-producing buildings → ExitObject returns 0 (no placement) | [doc: RALLY_POINTS §6 #6] | **Deferred follow-up.** No amphibious infantry in vanilla YR by default. Not in this design's scope. |
| 11 | Walk locomotor calls `PlaceInfantryInCell` on the destination cell each step; sub-cell is assigned as a walk destination (no snap) | [doc: INFANTRY_SUBCELL §"Walk Locomotor Integration"] [GHIDRA `0x0075C240`] | **Deferred follow-up.** Walk-loco scope, not spawn scope. |
| 12 | Random rotation table (`0x0081CC98`) consulted ONLY when `PlaceInfantryInCell` quadrant == 0; preference table (`0x0081CC84`) for quadrants 1-4 | [doc: INFANTRY_SUBCELL §"Random rotation table"] | **Deferred follow-up.** Lives inside `PlaceInfantryInCell` which the Rust port doesn't have. Walk-loco scope. |
| 13 | Initial body facing on Unlimbo: gamemd's ExitObject alt path facing arg not separately verified; Rust currently uses `facing = 64` | [doc: RALLY_POINTS §18 step 5] [UNKNOWN — needs RE] | **Deferred follow-up.** Keep `facing = 64` literal at the `spawn_object` call site. Flag for RE. |
| 14 | `cell_passable_for_infantry` rejects cells with `0x40` (building bit). For the *producing* building's own cells we need to bypass this — gamemd's `CanEnterCell` returns OK because infantry can occupy own-building foundation cells | [doc: INFANTRY_SUBCELL §"Occupancy Byte Bit Field"] | `find_infantry_spawn_cell_near_structure` does NOT call `cell_available_for_spawn` at all. Returns the foundation-center cell unconditionally. |

## Design

### Components

One new function in `production_spawn.rs`:

```
find_infantry_spawn_cell_near_structure(
    rules: &RuleSet,
    base_rx: u16,
    base_ry: u16,
    structure_id: &str,
) -> Option<(u16, u16)>
```

Returns `Some((base_rx + foundation_w / 2, base_ry + foundation_h / 2))`,
clamped to non-negative cell coords. `None` only if `rules.object(structure_id)`
returns `None` (unknown structure).

One dispatch branch in `find_spawn_cell_for_owner`:

```
for (_sid, bx, by, structure_id) in bases {
    let cell = match produced_category {
        ObjectCategory::Infantry =>
            find_infantry_spawn_cell_near_structure(rules, *bx, *by, structure_id),
        _ =>
            find_spawn_cell_near_structure(*bx, *by, structure_id, produced_category,
                rules, path_grid, &sim.occupancy, resolved_terrain, require_water),
    };
    if let Some(cell) = cell { return Some(cell); }
}
```

### Data Flow

1. Production queue tick (`production_queue.rs:489`) determines a unit is ready.
2. `find_spawn_cell_for_owner` is called with `produced_category =
   ObjectCategory::Infantry`.
3. Producer-candidate scan picks the active or fallback producing barracks.
4. New dispatch branch routes to `find_infantry_spawn_cell_near_structure`.
5. Returns the foundation-center cell `(bx + w/2, by + h/2)` of the barracks.
6. `production_queue.rs:521` calls `sim.spawn_object(type, owner, rx, ry,
   facing=64, …)`. `world_spawn.rs` infantry block assigns sub_cell=2 and
   the corresponding sub-x/sub-y lepton offset (unchanged).
7. If rally set, `production_queue.rs:574` issues
   `issue_move_command_with_layered` toward the rally point. The unit walks
   out through the foundation cells via the existing pathfinder.

### Interfaces / Contracts

- `find_spawn_cell_for_owner` keeps its signature
  `(sim, rules, owner, produced_category, path_grid, require_water) ->
  Option<(u16, u16)>`. No external consumer change.
- `find_infantry_spawn_cell_near_structure` is module-private (no `pub`).
- The function does not touch `sim.occupancy`, `path_grid`, or
  `resolved_terrain` — it accepts none of them as parameters. This is
  intentional: gamemd's infantry alt path performs no per-cell passability
  check at the spawn step.

### Error Handling

- `rules.object(structure_id)` returns `None` → return `None`. Caller refunds
  the unit cost (existing behavior at `production_queue.rs:513`).
- Foundation dimensions of zero (parser corruption) → produces the cell at
  the building's anchor TL. Acceptable degenerate fallback; matches the
  current `preferred_exit_offsets` default of `(2, 2)` foundation.

### Testing Strategy

Unit tests in `production_tests.rs`:

1. **`infantry_spawn_uses_foundation_center_cell`** — given a 3×3 barracks at
   `(10, 10)`, infantry spawns at `(11, 11)`, regardless of any `ExitCoord`
   value in INI. Test with `YABRCK` (`ExitCoord=-64,64,0`) to prove the
   ExitCoord is ignored.
2. **`infantry_spawn_ignores_exit_coord_even_with_large_value`** — barracks
   with synthetic `ExitCoord=2048,1024,0` (8 cells, 4 cells); infantry still
   spawns at foundation center.
3. **`infantry_spawn_handles_even_dimension_foundation`** — 4×4 barracks at
   `(10, 10)`, infantry spawns at `(12, 12)`. (Integer division: `4/2 = 2`.)
4. **`infantry_spawn_succeeds_when_foundation_center_blocked_by_building`** —
   the producer building itself occupies the foundation-center cell;
   spawn still succeeds (no `cell_available_for_spawn` check at the spawn
   step). Distinguishes from vehicle/aircraft path.
5. **`vehicle_spawn_path_unchanged`** — vehicle production from `GAWEAP`
   still uses the existing ExitCoord-as-cell-offset path (regression
   anchor for the deferred-scope vehicle fix).

Existing infantry tests in `production_tests.rs:658-732` that assert
ExitCoord-derived cells for infantry need to be replaced with the above.
Vehicle ExitCoord *parsing* test (`exit_coord = Some((512, 256, 0))`)
stays — that's testing the INI parser, not the spawn path.

### Determinism Considerations

- New code is pure integer arithmetic. No RNG, no float, no map-iteration
  order dependency.
- `producer_candidates_for_owner_category` ordering is unchanged.
- Rally-MoveTo path is unchanged.
- The deferred random-rotation work (item #12) will need the deterministic
  sim RNG (`sim.rng`), not `rand::thread_rng()` — but that's out of scope
  here.

## Architectural Decisions

- **Pattern followed:** sim/ helpers stay in their owning module
  (`production_spawn.rs`). One function, one responsibility. Dispatch by
  category at the entry point.
- **Pattern deviated from:** none in this scope. The existing
  `preferred_exit_offsets` + `find_spawn_cell_near_structure` machinery
  stays intact for vehicle/aircraft (still parity-wrong but scope-deferred).
- **Tech debt introduced:** none new. Tech debt acknowledged but
  scope-deferred: the vehicle/aircraft path's ExitCoord-as-cell-offset
  misuse remains, with the *same* function name `preferred_exit_offsets`
  serving the (now-single) vehicle/aircraft case. A follow-up brainstorm
  should redesign that with the correct gamemd semantics
  (`GetDockCellForObject` + barracks-flag-gated lepton-coord add).

## Alternatives Considered

- **Approach A — per-RTTI branch inside `preferred_exit_offsets`:**
  rejected because it merges two distinct gamemd dispatch paths (alt-path
  Unlimbo-at-center vs `GetDockCellForObject`) into one function. Same
  parity correctness but worse domain modeling.
- **Approach C — intercept at `production_queue.rs`:** rejected because
  it grows the already-large queue tick loop with logic that belongs in
  the spawn-selection module. The existing module separation exists
  precisely to keep that loop legible.
- **"Skip random rotation entirely":** explicitly offered and explicitly
  rejected by the user during brainstorm. Random rotation is a deferred
  follow-up, not a permanent drift.

## Deferred Follow-Ups (parity items NOT addressed by this design)

These items are tracked here so /write-plan and implementation do not
silently drop them. Each will need its own brainstorm + plan.

1. **Vehicle/aircraft `ExitCoord` semantics** (RALLY_POINTS §6 vehicle path,
   §8 `GetDockCellForObject`). Current Rust path treats `ExitCoord` as a
   cell offset for all categories; gamemd uses it as a lepton delta applied
   conditionally on barracks-flag cell matches. Affects every vehicle and
   aircraft spawn from a barracks-flag-bearing building.
2. **Walk-loco `PlaceInfantryInCell` port** — directional preference table
   `0x0081CC84` (5×4) + random rotation table `0x0081CC98` (4×4) + Mark/Unmark
   virtual call sequence. Currently the Rust port snaps infantry to the
   sub-cell anchor on movement completion (per `INFANTRY_SUBCELL §"Already
   Implemented"`); gamemd interpolates the walk through the sub-cell offsets.
   Visible as movement-pacing drift when 2+ infantry traverse the same cell.
3. **Amphibious infantry exit** (RALLY_POINTS §6 #6, `Type+0xE0D` gate).
   Currently no amphibious-infantry handling. Not a hot path in vanilla YR.
4. **Initial body facing for barracks-exit infantry** (RALLY_POINTS §18
   step 5). Rust uses `facing = 64`; gamemd's ExitObject alt-path facing
   arg was not separately RE'd. Needs Ghidra trace of `ExitObject_Main`
   at `0x443F54` to extract the literal passed to `Unlimbo`.
