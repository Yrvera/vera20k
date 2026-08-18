# TIBTRE Midpoint Forced Spawn After Fix Trace

Date: 2026-05-27

Scenario: A stock standard-YR `TIBTRE01` is live on a normal clear map cell. A deterministic setup makes the idle spawn probability roll succeed. Trace the path from the GameMD idle tick through the 22-frame terrain animation midpoint at stock `AnimationRate=3`, then `SpreadTiberium(force=true)`, then target placement as tiberium density/data `3`, and compare current Rust.

Scope is exactly this TIBTRE midpoint forced spawn. TIBTRE damage/removal, natural ore growth, harvest value, render pixel composition, save/load, and non-stock modded terrain are adjacent only.

## Evidence

- GameMD research: `TIBTRE_TERRAINCLASS_AI_TIMING_AND_RNG_GHIDRA_REPORT.md`, active standard YR path `TerrainClass::AI @ 0x0071C730`.
- GameMD research: `TIBTRE_RETAIL_SHP_FRAME_COUNTS_AND_MIDPOINT_TICKS_GHIDRA_REPORT.md`, stock retail `TIBTRE01/02/03` theater SHPs have `22` frames.
- GameMD research: `TIBTRE_CANACCEPTTIBERIUM_REJECTION_GATES_GHIDRA_REPORT.md`, `SpreadTiberium(force=1) -> CanPlaceTiberium`.
- GameMD research: `TIBTRE_PLACETIBERIUM_DENSITY_OVERLAY_QUEUE_EFFECTS_GHIDRA_REPORT.md`, `PlaceTiberium(type, 3)` writes `OverlayData=3`, constructs a random flat Riparius overlay, adds growth queue, and marks radar terrain dirty.
- GameMD research: `ORE_TIBERIUM_RNG_CLASSIFICATION_GHIDRA_REPORT.md`, TIBTRE probability uses raw `Random::Next`, direction uses `RandomRanged(0,7)`, empty placement uses `RandomRanged(0,11)`.
- INI: `ini/rulesmd.ini:28109-28121` stock `[TIBTRE01]` has `SpawnsTiberium=yes`, `IsAnimated=yes`, `AnimationRate=3`, `AnimationProbability=.003`, and `Immune=yes`.
- INI: `ini/artmd.ini:12653-12655` stock `[TIBTRE01]` has `Theater=yes` and `Foundation=1x1`.
- Current Rust: `src/sim/terrain_spawn.rs`, `src/app_init.rs`, `src/sim/ore_growth.rs`, `src/sim/overlay_grid.rs`, `src/sim/world/mod.rs`, `src/app_sim_tick.rs`.

## Pipeline

`TerrainClass::AI idle tick` -> probability hit -> start terrain animation frame 0 -> rate-3 frame timer -> midpoint frame 11 of 22 -> reset active state -> `SpreadTiberium(force=1)` -> random adjacent candidate scan -> `CanPlaceTiberium` accepts the first normal clear target -> `PlaceTiberium(type 0/Riparius, 3)` -> overlay/data/resource/growth/dirty state -> visible ore cell.

## Stage Verdicts

| Stage | GameMD output | Current Rust output | Verdict |
|---|---|---|---|
| Stock data and liveness | `[TIBTRE01]` is standard YR live terrain with `SpawnsTiberium=yes`, `IsAnimated=yes`, `AnimationRate=3`, `AnimationProbability=.003`; AI path is live, not TS legacy. | `seed_terrain_spawners` creates live terrain object state for every map terrain object and a derived spawner for `SpawnsTiberium && IsAnimated` at `src/sim/terrain_spawn.rs:579-644`. | PASS |
| Source-cell baseline | In this concrete clear-source scenario, `TerrainClass::Unlimbo` leaves no same-cell source ore, so forced spread defaults to tiberium type 0/Riparius. | App load clears same-cell tiberium source overlays/resources for spawning terrain at `src/app_init.rs:220-289`; stock clear source remains no-source/Riparius-like ore path. | PASS |
| Probability-hit branch | GameMD consumes one raw `Random::Next`, signed-abs `% 1_000_000`, scales by `1e-6`, then strict-compares against stored float `.003`; this scenario assumes the roll succeeds. | `raw_probability_sample` uses one raw `next_u32`, signed abs, `% 1_000_000`, and strict `<` against parsed micros at `src/sim/terrain_spawn.rs:76-94`; exact seed/raw remainder was not supplied or computed against GameMD. | UNCHECKED |
| Hit tick effect | On hit tick `H`, GameMD writes current frame `0`, arms duration `3`, and does not call `SpreadTiberium`. | Idle hit sets `Active { current_frame: 0, ticks_until_next_frame: 3 }` and returns `AnimationStarted`; `try_spawn_ore` is called only for `SpawnDue` at `src/sim/terrain_spawn.rs:158-189` and `:319-342`. | PASS |
| Active tick RNG | GameMD does not reroll probability while animation is active; direction RNG is delayed until midpoint. | Active branch only decrements/advances timer and does not call `roll_succeeds`; tests assert no RNG consumption on active non-midpoint tick at `src/sim/terrain_spawn.rs:841-860`. | PASS |
| Midpoint timing | With 22 frames and rate 3, midpoint is frame `11`; spawn call occurs on the 11th timer expiry, `H+33`. | `midpoint_frame = frame_count / 2`; current frame reaches `11` after 33 ticks and returns `SpawnDue`; test covers stock rate-3 `H+33` at `src/sim/terrain_spawn.rs:819-838`. | PASS |
| Midpoint reset before spread | GameMD resets active animation fields before calling `CellClass::SpreadTiberium(1)`. | `spawner.tick` sets phase back to `Idle` before returning `SpawnDue`; caller invokes `try_spawn_ore` after that at `src/sim/terrain_spawn.rs:186-189` and `:324-342`. | PASS |
| Tick integration | GameMD terrain AI runs during object AI and calls spread at the midpoint tick. | World tick calls `tick_terrain_spawners_stateful` after ore growth, passing overlay grid, growth queue, live object context, resolved terrain, overlay registry, and path grid at `src/sim/world/mod.rs:1624-1642`. Exact object scheduler ordering against all other GameMD object AI was not re-computed here. | UNCHECKED |
| Direction candidate draw | GameMD `SpreadTiberium(force=1)` consumes `RandomRanged(0,7)` once, then scans 8 neighbors from the random start. | `try_spawn_ore` uses `rng.next_range_u32(8)` once and scans `(start + i) % 8` over `ADJACENT_OFFSETS` at `src/sim/terrain_spawn.rs:379-390`. Exact cardinal offset order and exact chosen neighbor were not computed against GameMD. | UNCHECKED |
| Concrete clear target gate | For the first normal clear adjacent target: in playfield, no overlay, no blocking object/building, flat, buildable land, not bridge/rail, `AllowTiberium=true`; GameMD `CanPlaceTiberium` accepts. | `can_accept_tiberium` rejects existing resources/overlays, spawning terrain cells, invalid resolved terrain, non-`AllowTiberium`, non-flat, base-build-blocked, bridge flags, and visible live structures at `src/sim/terrain_spawn.rs:427-516`; the concrete clear target accepts. | PASS |
| Empty placement density/data | GameMD reaches `PlaceTiberium(type, 3)` and writes exact `OverlayData = 3`. | `place_tiberium_empty` inserts ore remaining `120 * 3 = 360` and writes overlay data `3` at `src/sim/terrain_spawn.rs:541-552`. | PASS |
| Overlay variant draw | GameMD flat new placement chooses Riparius flat variant with `RandomRanged(0,11)` from the type image range. | If `OverlayTypeRegistry` is present, Rust chooses from parsed `TIB01..TIB12` using `rng.next_range_u32(ids.len())` at `src/sim/terrain_spawn.rs:532-571` and `src/map/overlay_types.rs:299-325`; exact selected variant was not computed because no seed/target draw sequence was supplied. | UNCHECKED |
| Growth queue side effect | GameMD adds the new cell to `TiberiumClass::AddToGrowthQueue` immediately after overlay construction and before writing `OverlayData=3`, with priority `currentFrame + (signed_abs(Random::Next()) % 50)`, and does not add spread queue. | Rust enqueues one growth queue entry with the same priority formula at `src/sim/terrain_spawn.rs:554-556` and `src/sim/ore_growth.rs:169-188,348-361`, but the model is a Rust `OreGrowthState` queue and insertion occurs after `resource_nodes`/overlay data writes, not at the native per-type queue/write order. | FAIL |
| Dirty and radar side effects | GameMD dirties tactical terrain and calls `RadarClass::MarkTerrainDirty`, appending/deduping the cell and setting the radar dirty flag. | `OverlayGrid::place_overlay` marks overlay dirty at `src/sim/overlay_grid.rs:101-109`, and app drains dirty cells into render overlays/passability at `src/app_sim_tick.rs:690-738`; no matching call marks `Simulation::radar_terrain_dirty_cells` for this TIBTRE placement path (`src/sim/world/mod.rs:500-515`). | FAIL |
| Visible tactical ore | GameMD produces a visible Riparius ore overlay on the accepted adjacent cell, data byte `3`, on midpoint tick. | Current Rust mutates `OverlayGrid`, pushes a render overlay entry from dirty cells, and creates a resource node on the midpoint tick; exact pixel/palette composition was not traced here. | PASS |

## Findings

1. Growth queue side effect is structurally close but not GameMD-identical. Current Rust now inserts a native-shaped priority, but it writes resource/overlay state before queue insertion and stores it in a Rust `OreGrowthState`, not the per-type `TiberiumClass` queue. This can change later growth ordering or save/load-visible queue state. Evidence: GameMD report `TIBTRE_PLACETIBERIUM_DENSITY_OVERLAY_QUEUE_EFFECTS_GHIDRA_REPORT.md:78`; Rust `src/sim/terrain_spawn.rs:541-556`.

2. Radar terrain dirty is still not GameMD-identical for the spawn placement path. The tactical overlay becomes visible through `OverlayGrid` dirty drain, but `Simulation::radar_terrain_dirty_cells` is not marked when TIBTRE places ore, while GameMD calls `RadarClass::MarkTerrainDirty`. Evidence: GameMD report `TIBTRE_PLACETIBERIUM_DENSITY_OVERLAY_QUEUE_EFFECTS_GHIDRA_REPORT.md:70,147`; Rust `src/app_sim_tick.rs:690-738` and `src/sim/world/mod.rs:500-515`.

3. Exact RNG-output equality remains unchecked for this scenario. The call shapes are now largely aligned: raw probability on idle hit, no same-tick direction, `RandomRanged(0,7)` at midpoint, `RandomRanged(0,11)` for flat overlay, and one raw growth-queue priority draw. This trace did not compute a concrete GameMD seed stream and Rust seed stream to prove the exact target cell and overlay variant match.

## TS/YR Boundary

This is active standard Yuri's Revenge behavior. Stock `rulesmd.ini` enables `TIBTRE01` spawning and animation, retail theater SHPs provide the 22-frame image data, and the verified path is `TerrainClass::AI -> CellClass::SpreadTiberium(force=1) -> CellClass::CanPlaceTiberium -> CellClass::PlaceTiberium(type, 3)`. The Tiberium naming is engine terminology reused for RA2/YR ore; this is not a dormant TS weed or vein-only path.

## Adjacent Findings

- `src/rules/terrain_object_type.rs:26-32` has stale comments describing collapsed timing and `rng.next_range_u32(1_000_000)` probability use. The implementation no longer does that; this trace did not edit comments.
- Full natural ore growth/spread queue processing remains a separate trace target.
- Exact terrain sprite/palette/minimap pixel parity for the newly placed overlay remains a render trace target.

## Verdict Tally

PASS: 9 | FAIL: 2 | UNCHECKED: 4 | NOT-IMPLEMENTED: 0

Status: COMPLETE
