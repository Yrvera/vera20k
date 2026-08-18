# TIBTRE Source-Cell Unlimbo Clear Trace - 2026-05-27

Scenario: a stock YR map cell contains `[Terrain]` `TIBTRE01` and same-cell tiberium overlay/data from `[OverlayPack]` / `[OverlayDataPack]`.

Scope: map-load source-cell clearing only. This trace checks load ordering, `TerrainClass::Unlimbo` source-cell overlay/data writes, Rust `app_init` clearing, passability/resource recalculation, and terrain-object lifecycle ownership. It does not trace TIBTRE midpoint spawning, target placement, damage, savegame restore, or editor overlay tools.

## Pipeline

`[OverlayPack]` source ore stamped -> terrain section read -> `TerrainClass` constructor / Rust terrain seeding -> source-cell tiberium overlay/resource clear -> passability/resource metadata update -> live terrain object remains as lifecycle owner / derived spawner source.

## Evidence

- Active YR load order: fresh read-only Ghidra decompile of `ScenarioClass::Full_Init @ 0x00686B20` shows `Read_Map_Section_And_IsoMapPacks`, then `ReadMapOverlayPacks`, then all-cell `CellClass::RecalcAttributes`, then `TerrainClass__Read_Map_Section`, then tiberium growth/spread queue init.
- Active YR overlay stamp: fresh read-only Ghidra decompile of `ReadMapOverlayPacks @ 0x005FD2E0` decodes `[OverlayPack]`, constructs `OverlayClass` for non-`0xff` ids, then decodes `[OverlayDataPack]` and writes `CellClass+0x11E`.
- Active YR terrain construction: fresh read-only Ghidra decompile of `TerrainClass__Read_Map_Section @ 0x0071CA70` decodes modern keys as `rx = key % 1000`, `ry = key / 1000`, allocates `0xe0`, and calls `TerrainClass__Constructor`.
- Active YR Unlimbo: fresh read-only Ghidra decompile of `TerrainClass__Constructor @ 0x0071BB90` calls `TerrainClass__Unlimbo` at cell center `(rx*256+128, ry*256+128, z=0)`. `TerrainClass__Unlimbo @ 0x0071D000` calls `ObjectClass__Reveal`, increments all 8 neighbor `CellClass+0x122` bytes by `1`, then if source `Cell+0x44 != -1` and `OverlayTypeClass+0x2A9 != 0`, writes `Cell+0x44 = -1` and `Cell+0x11E = 0`.
- Stock YR data: `ini/rulesmd.ini` has `[TIBTRE01] SpawnsTiberium=yes`, `IsAnimated=yes`, `Immune=yes`; `[TIB01]` and sibling tiberium/gem overlays have `Tiberium=yes`; `[Tiberiums] 0=Riparius`.
- Prior research agrees: `TIBTRE_SOURCE_OVERLAY_TYPE_REACHABILITY_AFTER_UNLIMBO_GHIDRA_REPORT.md` lines 41-90 and 197-205; `TIBTRE_TERRAIN_OBJECT_LIFECYCLE_AND_SEEDING_GHIDRA_REPORT.md` lines 49-60 and 193-200.

## Stage Verdicts

| Stage | Concrete gamemd output | Current Rust output | Verdict |
|---|---|---|---|
| Stock source data | `TIBTRE01` is an active terrain type with `SpawnsTiberium=yes`; tiberium overlay ids have tiberium flag byte set. | `TerrainObjectType` parses `spawns_tiberium`; `OverlayTypeRegistry` exposes `flags(...).tiberium`. Relevant code: `src/rules/terrain_object_type.rs:78`, `src/map/overlay_types.rs:64`. | PASS |
| Initial source overlay exists before terrain placement | `ReadMapOverlayPacks` runs before `TerrainClass__Read_Map_Section`; source cell can contain overlay id plus data before `Unlimbo`. | Rust seeds resource nodes from map overlays at `src/app_init.rs:758` and creates `OverlayGrid` from map overlays at `src/app_init.rs:784` before the source-cell clear at `src/app_init.rs:801`. | PASS |
| Terrain object lifecycle owner | GameMD allocates one live `TerrainClass` object and the spawn behavior is owned by live terrain object AI, not by an independent spawner list. | Rust now creates `TerrainObjectState` and `terrain_object_cells`, while `terrain_spawners` is documented and stored as a derived index. Relevant code: `src/sim/terrain_spawn.rs:592`, `src/sim/terrain_spawn.rs:604`, `src/sim/production/production_types.rs:214`. | PASS |
| Source-cell overlay/data clear | GameMD writes source `Cell+0x44=-1` and `Cell+0x11E=0` when the source overlay type has `+0x2A9 != 0`. For a stock tiberium overlay this means no overlay and data `0`. | `clear_tiberium_source_cells_for_spawning_terrain` removes the source `resource_nodes` entry, clears the mutable overlay grid when registry flags say `tiberium`, and `OverlayGrid::clear_overlay` writes `overlay_id=None` / `overlay_data=0`. Relevant code: `src/app_init.rs:240`, `src/app_init.rs:247`, `src/sim/overlay_grid.rs:93`. | PASS |
| Passability/resource recalculation | GameMD decompile proves the overlay/data clear and prior all-cell `RecalcAttributes`; this trace did not compute the exact post-clear cached cell `LandType`, speed costs, radar/resource queue, or dirty-cell state after `Unlimbo`. | Rust explicitly calls `recalc_overlay_passability` for cleared source cells on both resolved terrain grids, restoring base land type/speed costs while preserving terrain-object blocking. Relevant code: `src/app_init.rs:266`, `src/sim/overlay_grid.rs:184`, `src/sim/overlay_grid.rs:242`. | UNCHECKED |
| Exact Unlimbo placement mechanism | GameMD clears as part of `TerrainClass::Unlimbo` during object placement, immediately after reveal and neighbor counter increments. | Rust performs source-cell clearing in an app-level reconciliation helper after `seed_terrain_spawners` and overlay-grid creation, not inside a terrain-object Unlimbo/placement operation. Final stock map-load source-cell overlay/resource output matches, but the owner/order is not byte-mechanism equivalent. Relevant code: `src/app_init.rs:220`, `src/app_init.rs:801`. | FAIL |
| Regression coverage for this exact fixture | GameMD numerical outputs for this scenario are known for overlay id/data clear (`-1`, `0`) from decompile. | Existing `terrain_object` and `terrain_spawn` suites pass, but there is no focused executable fixture named for same-cell `[OverlayPack]` ore plus `[Terrain] TIBTRE01` map load. | UNCHECKED |

## Findings

### FAIL - Unlimbo ownership/order drift

Player-visible risk is low for ordinary startup because Rust clears before gameplay ticks begin, but the mechanism is still not GameMD-identical. In GameMD, the clear is an immediate `TerrainClass::Unlimbo` side effect owned by the live terrain object placement path. Rust clears through `app_init` after spawner/live-state seeding, so future live terrain placement, editor placement, or alternate map-load paths can miss the same side effect unless they also remember to call the helper.

Root cause: `clear_tiberium_source_cells_for_spawning_terrain` is a map-load reconciliation pass, not a terrain-object placement primitive.

### UNCHECKED - Exact cached passability/resource dirtiness after GameMD clear

GameMD certainly clears `Cell+0x44` and `Cell+0x11E`, and it ran all-cell `RecalcAttributes` before terrain construction. This trace did not compute whether any cached `CellClass` land type, speed-cost, radar, zone, or dirty flags are recomputed after the clear. Rust does recompute overlay passability and land/speed metadata after clearing; this may be the right player-facing final state, but exact post-Unlimbo byte equality is unproven.

## Verification Commands

- `cargo test -q terrain_object --lib` - passed, 11 tests.
- `cargo test -q terrain_spawn --lib` - passed, 20 tests.

These are supporting checks only; neither is a dedicated same-cell map fixture.

## Verdict Tally

PASS: 4 | FAIL: 1 | UNCHECKED: 2 | NOT-IMPLEMENTED: 0

## Top Player-Visible FAIL / NOT-IMPLEMENTED Findings

1. Stage: exact Unlimbo placement mechanism - player-visible difference: future live/editor/alternate terrain placement can leave source-cell ore unless it duplicates the app-init helper - our file: `src/app_init.rs:220` / `src/app_init.rs:801` - gamemd evidence: `TerrainClass__Unlimbo @ 0x0071D000` clears source `Cell+0x44` and `Cell+0x11E` during object placement.

## Adjacent Findings

- Source-overlay type propagation at later TIBTRE spawn is real but out of scope here. For stock same-cell map ore, GameMD clears the source overlay before any midpoint spawn, so stock source type defaults to tiberium type `0` (`Riparius`).
- Rust has no focused same-cell overlay plus `[Terrain] TIBTRE01` map-load regression fixture. Adding one would turn the static source-clear claim into an executable guard.
