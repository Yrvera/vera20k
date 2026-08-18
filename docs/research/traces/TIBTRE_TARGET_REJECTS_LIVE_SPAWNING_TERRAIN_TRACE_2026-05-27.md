# TIBTRE Target Rejects Live Spawning Terrain Trace - 2026-05-27

## Scenario

Two stock TIBTRE terrain objects are adjacent. Source TIBTRE A attempts to spawn
tiberium onto target cell B, occupied by stock TIBTRE B. This trace covers only
the target-cell rejection for a live `SpawnsTiberium=yes` terrain object.

Concrete coordinates used for computation:

- Source TIBTRE A: `(10, 10)`
- Target TIBTRE B: `(11, 10)`
- Target offset from source: `(1, 0)`
- Target type: `TIBTRE01`
- Stock data: `SpawnsTiberium=yes`, `IsAnimated=yes`

## Sources

- Verified GameMD report:
  `docs/research/TIBTRE_CANACCEPTTIBERIUM_REJECTION_GATES_GHIDRA_REPORT.md`
  lines 47-74, 101-110, 134-150, 153-170, 183-194.
- Stock INI: `ini/rulesmd.ini` lines 28109-28143 show stock
  `TIBTRE01/02/03` set `SpawnsTiberium=yes` and `IsAnimated=yes`.
- Rust:
  - `src/rules/terrain_object_type.rs` lines 78-84 parse
    `SpawnsTiberium`, `IsAnimated`, and lifecycle-relevant type data.
  - `src/sim/terrain_spawn.rs` lines 319-341 pass both current spawner cells
    and derived spawning terrain cells into target validation.
  - `src/sim/terrain_spawn.rs` lines 392-401 call `can_accept_tiberium`
    before placement.
  - `src/sim/terrain_spawn.rs` lines 427-466 reject cells present in
    `spawning_terrain_cells` or `spawner_cells`.
  - `src/sim/terrain_spawn.rs` lines 579-645 seed live terrain objects,
    `terrain_object_cells`, `tiberium_spawning_terrain_cells`, and animated
    `terrain_spawners`.
  - `src/sim/world/mod.rs` lines 1627-1642 passes
    `production.tiberium_spawning_terrain_cells` into the terrain-spawn tick.
  - `src/sim/production/production_types.rs` lines 215-229 define
    `terrain_spawners` as a derived index and
    `tiberium_spawning_terrain_cells` as the broader live spawning-terrain
    cell set.

## Pipeline

1. `TerrainClass::AI` reaches midpoint for source TIBTRE A.
2. GameMD calls `CellClass::SpreadTiberium(force=1)` on source cell `(10,10)`.
3. The neighbor loop considers target cell `(11,10)`.
4. GameMD calls `CellClass::CanPlaceTiberium` before placement.
5. `CanPlaceTiberium` scans the target cell object list and finds a live
   `TerrainClass` object whose `TerrainTypeClass+0x2B1 SpawnsTiberium` byte is
   nonzero.
6. GameMD rejects target cell `(11,10)`, so `PlaceTiberium(type, 3)` is not
   called for that cell.
7. Rust `tick_terrain_spawners_stateful` reaches `try_spawn_ore`; the target
   candidate is passed to `can_accept_tiberium` before `place_tiberium_empty`.
8. Rust rejects target cell `(11,10)` because the cell is present in the
   derived `tiberium_spawning_terrain_cells` set, and for stock animated
   TIBTRE it is also present in the local `spawner_cells` snapshot.

## Stage Table

| Stage | Concrete value | GameMD | Rust | Verdict |
|---|---:|---|---|---|
| Stock type data | `TIBTRE01 SpawnsTiberium=true, IsAnimated=true` | Active in standard YR per verified report and stock INI | Parsed from INI booleans | PASS |
| Live target object exists | target `(11,10)` has live TIBTRE object | Target cell object list contains live `TerrainClass` | `seed_terrain_spawners` inserts `terrain_objects` and `terrain_object_cells` | PASS |
| Derived spawning-terrain index | target `(11,10)` should reject as spawning terrain | Reject is based on live `TerrainClass` type byte `SpawnsTiberium != 0` | `seed_terrain_spawners` inserts `(11,10)` into `tiberium_spawning_terrain_cells` when `t.spawns_tiberium` is true | PASS |
| Target predicate result | `CanPlaceTiberium((11,10))` | `false` at terrain-object gate | `can_accept_tiberium((11,10)) == false` because the derived set contains the cell | PASS |
| Placement side effect on target | ore/resource added to `(11,10)` | `0` placements on this rejected target cell | `0` placements on this rejected target cell because `place_tiberium_empty` is skipped | PASS |
| Exact object-list order | order of rejection checks | GameMD checks object-list terrain after bounds, flags, and live-building branch | Rust checks derived spawning-terrain/spawner sets before resource, overlay, resolved-terrain, and live-building checks | UNCHECKED |

## Verdict

For this exact stock two-TIBTRE target cell, the current Rust behavior matches
the GameMD player-visible result: target cell `(11,10)` is rejected and receives
no tiberium.

The mechanism is not proven equivalent for every possible mixed-occupancy case
because Rust uses a synchronized derived set rather than scanning the target
cell object list in the same order as GameMD. That ordering difference is not
player-visible in this concrete scenario because both paths return `false`
before placement and produce zero target-cell resource/overlay writes.

## Entry Points Checked

- Map terrain load: `seed_terrain_spawners` populates live terrain object state
  and the derived spawning-terrain rejection set.
- Terrain spawn tick: `tick_terrain_spawners_stateful` snapshots spawner cells
  and passes the broader `tiberium_spawning_terrain_cells` set.
- Target validation: `try_spawn_ore` calls `can_accept_tiberium` before any
  placement side effect.
- Placement: `place_tiberium_empty` is unreachable for target `(11,10)` once
  the rejection predicate returns false.

## Adjacent Findings

- The broader live-building exception gate is outside this trace. Current Rust
  has a live structure gate, but this scenario contains no building.
- Exact direction-label mapping is outside this trace. The target candidate was
  traced assuming the neighbor loop considers `(1,0)`; both engines use an
  eight-neighbor scan and reject this candidate when considered.
- Full proof that every terrain lifecycle mutation keeps
  `tiberium_spawning_terrain_cells` synchronized is outside this trace. The
  current removal helpers do remove the cell on limbo/destruction, but this run
  did not trace removal.

## Verdict Tally

PASS: 5 | FAIL: 0 | UNCHECKED: 1 | NOT-IMPLEMENTED: 0

