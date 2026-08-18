# Tiberium Trees Stock Gameplay Design

## Goal

Implement stock Yuri's Revenge TIBTRE tiberium spawning parity by replacing Rust's immediate additive ore spawn with a delayed terrain-animation spawn that uses GameMD-shaped placement validation and tiberium queue side effects.

## Architecture Context

Current Rust owns TIBTRE spawning in `src/sim/terrain_spawn.rs`. `seed_terrain_spawners` creates one spawner state per map terrain object whose terrain type has `SpawnsTiberium=yes` and `IsAnimated=yes`. `tick_terrain_spawners` currently rolls probability and immediately calls `try_spawn_ore`, which scans adjacent cells and then calls `place_tiberium_additive`.

The current implementation is intentionally narrow, but it collapses multiple GameMD stages into one tick: `TerrainClass::AI` animation arming, midpoint spawn, `SpreadTiberium(force=true)`, `CanPlaceTiberium`, `PlaceTiberium(type, 3)`, overlay mutation, and growth-queue insertion. That collapse creates player-visible parity gaps: ore appears too early, existing ore grows when GameMD would skip it, overlay data is wrong, and terrain/building/bridge/theater gates are absent.

Relevant existing boundaries:

- `sim/` owns deterministic gameplay state and must not depend on `render/`, `ui/`, `sidebar/`, `audio/`, or `net/`.
- `ProductionState` owns `resource_nodes`, `ore_growth_config`, `ore_growth_state`, and `terrain_spawners`.
- `OverlayGrid` owns visible overlay id/data and dirty cells.
- `ResolvedTerrainGrid` already exposes current/final terrain facts such as bridge flags and slope-derived details, but does not expose `AllowTiberium`.
- `TilesetLookup` parses theater `Morphable`, but not `AllowTiberium`.
- `SimRng` exposes raw `next_u32`, which is needed for the probability roll and queue-priority RNG.
- `world_hash.rs` hashes `terrain_spawners`, so any new spawner animation state must be included.

The design should align with the broader `2026-05-23-yr-tiberium-boundary-design.md` direction, but it should not require a full tiberium subsystem rewrite before TIBTRE parity can land.

## Impact Analysis

Touched modules:

- `src/sim/terrain_spawn.rs`: replace immediate/additive spawn with a stateful delayed animation owner plus GameMD-shaped placement.
- `src/sim/production/production_types.rs`: extend `TerrainSpawnerState` and possibly add tiberium placement/growth queue state references.
- `src/sim/world/mod.rs`: pass a mutable terrain spawner map and a richer validation/placement context into the tick.
- `src/sim/world/world_hash.rs`: hash active animation frame/timer state and any queue additions that affect future deterministic output.
- `src/map/theater.rs`: parse `[TileSetNNNN] AllowTiberium` with default false.
- `src/map/resolved_terrain.rs`: expose a final resolved-tile `allows_tiberium` cell fact.
- `src/map/bridge_facts.rs`: existing `0x100 | 0x400` constants become placement blockers.
- `src/rules/terrain_object_type.rs`: preserve `AnimationRate` and probability, and make frame-count handoff explicit.
- `src/rules/object_type.rs`: building gate needs `InvisibleInGame` and, for modded parity, inherited `Invisible=`.
- `src/map/overlay_types.rs` or a future `rules/tiberium` module: provide tiberium type to flat overlay variant ranges.
- `src/sim/ore_growth.rs`: add the new-cell growth-queue insertion path, or provide a compatibility queue hook until the full YR tiberium boundary replaces the scanner.
- `src/app_init.rs`: map-load order must model terrain unlimbo clearing same-cell tiberium overlays/resources.

Risk areas:

- RNG order: probability roll, neighbor start, overlay variant, and growth queue priority must consume RNG in the same order for the same accepted spawn.
- Tick order: TIBTRE probability hit cannot place ore until the midpoint animation frame.
- State hashing/serde: active animation state and queue membership influence future output and must be deterministic.
- App initialization order: current Rust seeds resources before terrain spawners and initializes `OverlayGrid` later; source-cell overlay clear must update all relevant stores.
- Placement gates: using `PathGrid::is_walkable` as a proxy would preserve the wrong behavior.
- Test churn: existing tests currently assert known-wrong immediate/additive behavior and must be replaced.

## Chosen Approach

Approach 1: placement context plus native-compatible queue hook.

Keep TIBTRE logic in `sim::terrain_spawn`, but stop making it a self-contained ore adder. Instead, the terrain spawn tick receives a compact `TerrainSpawnContext` containing only the deterministic gameplay facts needed to reproduce the GameMD-visible behavior:

- resolved terrain facts,
- overlay/resource occupancy,
- live terrain-object blockers,
- live building blockers and invisibility exceptions,
- tiberium type and overlay variant metadata,
- `OverlayGrid` and resource mutation handles,
- the growth queue insertion surface,
- and `SimRng`.

`TerrainSpawnerState` becomes an actual terrain-animation state machine. A probability hit only starts animation. The spawn happens later when the active frame reaches the loaded asset midpoint.

This approach keeps `sim/terrain_spawn.rs` testable, avoids pushing all logic into `world/mod.rs`, and gives every contract item a concrete owner without adding render or asset dependencies to `sim`.

## Tiny-Detail Ledger

- TIBTRE rolls probability only while idle. Source: `TIBTRE_TERRAINCLASS_AI_TIMING_AND_RNG_GHIDRA_REPORT.md`; contract row "Ore appears after the terrain tree animation reaches its midpoint".
- The probability roll uses raw `Random::Next`, signed abs/mod `1_000_000`, float scale by `1e-6`, and strict `< AnimationProbability`. Source: `TerrainClass::AI @ 0x0071C730`; contract evidence baseline.
- A hit starts the animation at frame 0 and does not spawn same tick. Source: `TIBTRE_TERRAINCLASS_AI_TIMING_AND_RNG_GHIDRA_REPORT.md`.
- Spawn fires when current frame equals loaded frame count / 2. Stock retail TIBTRE SHPs are 22 frames and stock `AnimationRate=3`, so stock spawn occurs 33 logic ticks after hit. Source: `TIBTRE_RETAIL_SHP_FRAME_COUNTS_AND_MIDPOINT_TICKS_GHIDRA_REPORT.md`; `ini/artmd.ini` / `art.ini`.
- `SpreadTiberium(1)` passes `force=true`, not tiberium type 1. Source: `TIBTRE_SPREADTIBERIUM_FORCE_TYPE_AND_FLAG_GATE_GHIDRA_REPORT.md`.
- Forced TIBTRE spawning bypasses `TiberiumSpreads`, but still derives/defaults a tiberium type. Source: same force/type report.
- Normal stock terrain unlimbo clears same-cell tiberium overlays before later source-type resolution, so stock TIBTRE defaults to type 0/Riparius. Source: `TIBTRE_SOURCE_OVERLAY_TYPE_REACHABILITY_AFTER_UNLIMBO_GHIDRA_REPORT.md`; `TerrainClass::Unlimbo @ 0x0071D000`.
- Exotic save/editor/direct-write source-overlay propagation is real but out of stock scope. Source: source-overlay reachability report; contract row marked `BLOCKED`.
- Neighbor scan starts from a random adjacent direction and checks up to 8 neighbors. Source: `TIBTRE_CANACCEPTTIBERIUM_REJECTION_GATES_GHIDRA_REPORT.md`.
- Any existing overlay, including existing ore/gems/walls/crates, rejects the target; TIBTRE does not grow existing ore. Source: `CellClass::CanPlaceTiberium`; contract row "existing ore is skipped, not grown".
- Target validation rejects bridge raw flags `0x100 | 0x400`. Source: `CELL_FLAGS_0X500_TIBTRE_PLACEMENT_SEMANTICS_GHIDRA_REPORT.md`; existing Rust constants in `bridge_facts.rs`.
- Target validation rejects live visible buildings, but permits the building branch for `InvisibleInGame=yes` / `Invisible=yes` exception cases. Source: `TIBTRE_BUILDING_EXCEPTION_BYTES_0XC9A_0X1701_GHIDRA_REPORT.md`.
- Target validation rejects live `SpawnsTiberium=yes` terrain objects. Source: `TIBTRE_CANACCEPTTIBERIUM_REJECTION_GATES_GHIDRA_REPORT.md`.
- Target validation requires buildable land, flat slope, no overlay, and in-range theater tile `AllowTiberium=true`; absent `AllowTiberium` defaults false. The verified binary fallback lets invalid/out-of-range tile indices pass only this final tile gate, so normal resolved in-range cells must still require the flag. Source: `ALLOWTIBERIUM_THEATER_READER_AND_RUST_SURFACE_GHIDRA_REPORT.md`; `TIBTRE_CANACCEPTTIBERIUM_REJECTION_GATES_GHIDRA_REPORT.md`.
- New TIBTRE cells write `OverlayData=3`. Source: `TIBTRE_PLACETIBERIUM_DENSITY_OVERLAY_QUEUE_EFFECTS_GHIDRA_REPORT.md`.
- New TIBTRE cells randomize a flat overlay variant from the selected tiberium type image range. Source: same PlaceTiberium report.
- New TIBTRE cells enter the growth queue immediately with priority `currentFrame + Random::Next() % 50` after overlay construction and before the final `OverlayData=3` write; they do not enter the spread queue from placement. Source: same PlaceTiberium report.
- New placement dirties overlay/tactical/radar state; Rust must publish equivalent overlay dirty effects without `sim` depending on render. Source: same PlaceTiberium report; `OverlayGrid` dirty-cell API.

## Design

### Components

#### Terrain spawner state

Extend `TerrainSpawnerState` from a passive probability record into a deterministic terrain animation record:

- `cell`
- `type_ref`
- exact parsed probability form plus a helper that reproduces raw `Random::Next`, signed abs/mod `1_000_000`, double `1e-6`, and strict comparison against the stored float
- `animation_rate_ticks`
- `frame_count`
- `midpoint_frame`
- `state: Idle | Active`
- active state fields:
  - `current_frame`
  - `ticks_until_next_frame` or equivalent timer deadline

The state stores only sim-safe numeric data. It does not store SHP handles or render asset references. App initialization resolves frame counts from loaded retail assets and passes the number into sim seeding.

#### Terrain spawn context

Introduce a short-lived context object for `tick_terrain_spawners`:

- `resolved_terrain`
- `overlay_grid`
- `resource_nodes`
- `terrain_object_blockers`
- `building_blockers`
- `tiberium_metadata`
- `growth_queue`
- `current_frame`
- `rng`

The context avoids a long positional argument list and makes the GameMD placement contract explicit. It remains a sim-level interface; no render, UI, audio, or app objects cross the boundary.

#### Placement validator

Create a TIBTRE-specific validation function shaped like `CanPlaceTiberium` for this call path. It should return a structured rejection reason in tests/debug builds, while release gameplay only needs the boolean.

Validation order should be documented and tested around observable effects. The important externally visible result is which neighbor wins after the random start, so the function must reject the same cells as GameMD before accepting the first valid candidate.

The validator checks:

- target inside playable map/local bounds,
- bridge mask does not contain `0x100 | 0x400`,
- no live visible building except the verified invisibility exceptions,
- no live `SpawnsTiberium=yes` terrain object,
- land type buildable,
- no overlay/resource occupancy,
- flat slope,
- final resolved in-range tile allows tiberium, with the binary out-of-range tile-index fallback documented separately from normal map cells.

#### Tiberium metadata

Add enough metadata to choose the stock type 0/Riparius flat overlay variant and value/density semantics:

- `[Tiberiums]` index to named type,
- type name to `Image`,
- `Image` to flat overlay variant range,
- stock value/density defaults needed by `resource_nodes`,
- mapping overlay id back to tiberium type for future source-overlay propagation.

For this stock TIBTRE design, normal source overlay propagation remains deferred. The metadata interface should include a future hook, but the implementation should default stock TIBTRE source to type 0 after unlimbo clearing.

#### Growth queue hook

Add a narrow insertion API:

```text
enqueue_tiberium_growth(type_id, cell, priority_frame)
```

If the full YR queue subsystem is not yet implemented, this hook may initially coexist with the current `OreGrowthState`, but it must still store deterministic queue membership and priority so the TIBTRE side effect is not lost. The later full tiberium boundary can take ownership of this state without changing the TIBTRE call site.

The hook must consume the queue-priority RNG only after a new overlay object has been constructed for an accepted empty cell. The native order is overlay variant RNG, overlay construction, growth-queue insertion and queue-priority RNG, then the final `OverlayData=3` write.

#### Source-cell unlimbo clear

During map load/seeding, live `SpawnsTiberium` terrain objects clear same-cell tiberium overlay/resource state. This must affect:

- map/resource seeding state,
- `OverlayGrid`,
- any source-type lookup used by TIBTRE.

The cleanest integration is to initialize overlay/resource state in an order that lets terrain object placement clear both stores exactly once. If app initialization cannot be reordered safely, add an explicit post-terrain-unlimbo reconciliation pass that removes same-cell tiberium overlays/resources for live TIBTRE terrain objects before gameplay starts.

### Interfaces / Contracts

`tick_terrain_spawners` should move from an immutable spawner view to a mutable state transition:

```text
tick_terrain_spawners(spawners: &mut BTreeMap<Cell, TerrainSpawnerState>, ctx: TerrainSpawnContext)
```

Contract:

- Idle spawners roll probability.
- Failed rolls do not mutate state except RNG consumption.
- Successful rolls enter active animation state only.
- Active spawners do not roll probability.
- Active spawners advance one frame per `AnimationRate`.
- When `current_frame == midpoint_frame`, they reset active animation state to idle, then attempt one forced spread.
- Whether placement succeeds or fails, the tree is already idle before the forced spread call, matching the active animation reset contract.

`place_tibtre_tiberium` contract:

- only accepts empty target cells that passed validation,
- constructs the selected overlay id,
- writes matching resource state,
- marks overlay dirty,
- enqueues growth queue,
- writes final `OverlayData=3`,
- does not enqueue spread queue,
- consumes overlay-variant RNG and growth-priority RNG in the verified order.

### Data Flow

1. App/rules load terrain object definitions, art data, overlays, tiberium type metadata, and theater tilesets.
2. Theater parsing stores `AllowTiberium` per tileset with default false.
3. Resolved terrain stores `allows_tiberium` from the final resolved tile for normal in-range map cells.
4. Map overlay/resource state is built.
5. Terrain object unlimbo/seeding clears same-cell tiberium overlays/resources for live spawning trees.
6. TIBTRE spawners are seeded with source cell, probability, animation rate, loaded frame count, midpoint frame, and default type id 0.
7. Each sim tick mutates spawner state:
   - idle probability roll,
   - active frame timer,
   - midpoint reset followed by forced spread attempt.
8. A successful forced spread validates neighbors from the random start and places exactly one new cell.
9. Placement mutates resource/overlay state, queues growth, and publishes dirty cells for app/render layers.
10. World hash covers the resulting state.

### Error Handling

Missing TIBTRE frame count or missing tiberium overlay metadata should not silently create resource-only ore. That would hide a visible parity failure.

Recommended behavior:

- In strict/dev builds, log or surface a load-time diagnostic naming the missing terrain type or metadata.
- In gameplay, skip spawning for that malformed terrain type rather than creating invisible/economy-only ore.
- Tests should assert that missing overlay metadata does not mutate `resource_nodes` without an overlay.

### Testing Strategy

Replace wrong current tests with targeted parity tests:

- `tibtre_probability_hit_does_not_spawn_same_tick`
- `tibtre_stock_rate3_spawns_33_ticks_after_probability_hit`
- `tibtre_active_animation_suppresses_probability_rolls`
- `tibtre_force_spawn_ignores_tiberium_spreads_false`
- `tibtree_unlimbo_clears_same_cell_tiberium_overlay_before_source_type_resolution`
- `tibtre_stock_source_without_overlay_spawns_riparius_type_zero`
- `tibtre_spawn_skips_existing_ore_neighbors_instead_of_growing_them`
- `tibtre_places_nothing_when_all_neighbors_have_overlays`
- `tibtre_spread_rejects_structural_bridge_cell_even_if_other_gates_pass`
- `tibtre_spread_rejects_destroyed_bridge_marker_cell_even_when_ground_walkable`
- `tibtre_spread_rejects_live_visible_building_cell`
- `tibtre_spread_allows_invisible_in_game_lamp_building_if_other_gates_pass`
- `tibtre_spread_rejects_any_spawns_tiberium_terrain_object_cell`
- `theater_parse_allow_tiberium_defaults_false`
- `resolved_terrain_allow_tiberium_uses_final_tile`
- `tibtre_spread_rejects_tile_without_allow_tiberium`
- `tibtre_probability_uses_float_strict_less_boundary`
- `tibtre_probability_uses_raw_next_not_random_ranged`
- `tibtre_spread_rejects_sloped_candidate`
- `tibtre_new_cell_overlay_data_is_three_not_stock_level_minus_one`
- `tibtre_new_cell_randomizes_riparius_flat_overlay_variant`
- `tibtre_new_cell_enqueues_growth_queue_not_spread_queue`
- `tibtre_growth_queue_insert_occurs_before_overlay_data_write`
- `tibtre_new_cell_marks_overlay_dirty_for_passability_and_minimap`
- world-hash tests proving active frame/timer and queued growth priority affect deterministic state.

Use fixed-seed `SimRng` tests for RNG ordering. Candidate scan tests should set up multiple adjacent cells where only one rejection difference changes the accepted target, because that catches proxy validators such as `PathGrid::is_walkable`.

## Architectural Decisions

- Keep ownership in `sim::terrain_spawn`, not app/render. This follows the existing deterministic sim pattern.
- Use a context object instead of moving the entire implementation into `World`. This keeps testing localized and prevents `world/mod.rs` from absorbing another gameplay subsystem.
- Add `AllowTiberium` to map/theater resolution rather than recomputing it during sim ticks. Placement needs a simple cell fact, and theater parsing already owns tile metadata.
- Add a narrow growth queue hook now. This avoids hiding a verified `PlaceTiberium` side effect while leaving the broader YR tiberium queue replacement to the existing tiberium boundary design.
- Treat source-overlay propagation as a future hook, not a stock shortcut. Normal stock gameplay clears the source overlay before TIBTRE uses it; save/editor/direct-write states need separate research before implementation.
- Fail closed when visual overlay metadata is missing. Resource-only ore would be worse than no spawn because the player would see economy behavior without the matching map state.

## Alternatives Considered

### World-owned implementation

Moving all TIBTRE logic into `Simulation`/`World` would make it easy to inspect entities, occupancy, terrain, overlays, and resource state. It was rejected because it would enlarge `world/mod.rs`, reduce isolated testability, and make future tiberium-boundary extraction harder.

### Partial parity pass

Fixing delayed timing and empty-cell placement while deferring growth queue, overlay variants, and some gates would be smaller. It was rejected because it leaves known player-visible parity drift in normal play: future growth timing and visible overlay choice can differ after a successful TIBTRE spawn.

### Full tiberium subsystem first

Replacing the entire ore/growth/spread/harvest subsystem before TIBTRE would be architecturally clean, and it matches the existing YR tiberium boundary design. It was rejected as a prerequisite because TIBTRE can use a narrow queue-compatible interface now without blocking on the full subsystem.
