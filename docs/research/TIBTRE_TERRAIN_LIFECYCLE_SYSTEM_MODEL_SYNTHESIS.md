# TIBTRE Terrain Lifecycle System Model Synthesis

**Date:** 2026-05-27  
**Mode:** model-synthesis with bounded gap queue  
**System:** TIBTRE / `SpawnsTiberium` terrain objects  
**Included:** map load, live terrain-object lifecycle, animation/RNG spawn timing, source-cell overlay clearing, target placement, current Rust state.  
**Non-scope:** full terrain damage implementation, trigger/script deletion matrix, binary savegame serialization, TIBTRE light rendering.

## Verdict

The stock gameplay model is implementation-safe for TIBTRE spawning and map-load source-cell clearing. The broader terrain-object lifecycle is only partially represented in Rust: current Rust has a detached spawner map keyed by cell, not live `TerrainClass` objects with full damage/limbo/occupation/save semantics.

## Claim Table

| Claim | Best evidence | Status | Active in YR | Safe? |
|---|---|---|---|---|
| TIBTRE is a normal live `TerrainClass` object loaded from `[Terrain]`. | `TIBTRE_TERRAIN_OBJECT_LIFECYCLE_AND_SEEDING_GHIDRA_REPORT.md`, `0x0071CA70`, `0x0071BB90` | confirmed | yes | IMPLEMENTATION_SAFE |
| Spawning is owned by `TerrainClass::AI`, not AnimClass or a global abstract emitter. | Ghidra spot-check `0x0071C730`; `TIBTRE_TERRAINCLASS_AI_TIMING_AND_RNG_GHIDRA_REPORT.md` | confirmed | yes | IMPLEMENTATION_SAFE |
| Idle AI rolls `Random::Next`, signed-abs `% 1_000_000`, scale `1e-6`, strict `< AnimationProbability`. | Ghidra spot-check `0x0071C730` | confirmed | yes | IMPLEMENTATION_SAFE |
| Probability hit starts animation at frame 0; spawn happens later at image frame count / 2. | `TIBTRE_RETAIL_SHP_FRAME_COUNTS_AND_MIDPOINT_TICKS_GHIDRA_REPORT.md`, `0x0071C730` | confirmed | yes | IMPLEMENTATION_SAFE |
| `SpreadTiberium(1)` argument is force=true, not tiberium type 1. | `TIBTRE_SPREADTIBERIUM_FORCE_TYPE_AND_FLAG_GATE_GHIDRA_REPORT.md` | confirmed | yes | IMPLEMENTATION_SAFE |
| Standard map-load same-cell tiberium overlay under TIBTRE is cleared by `Unlimbo`. | Ghidra spot-check `0x0071D000`; `TIBTRE_SOURCE_OVERLAY_TYPE_REACHABILITY_AFTER_UNLIMBO_GHIDRA_REPORT.md` | confirmed | yes | IMPLEMENTATION_SAFE |
| Stock TIBTRE source type defaults to type 0/Riparius because the source overlay was cleared. | `TIBTRE_SOURCE_OVERLAY_TYPE_REACHABILITY_AFTER_UNLIMBO_GHIDRA_REPORT.md`, `0x00483780`, `0x005FDD20` | confirmed | yes | IMPLEMENTATION_SAFE |
| Target placement uses `CanPlaceTiberium`: empty overlay, flat, buildable, `AllowTiberium`, bridge mask, live-building exception, no `SpawnsTiberium` terrain target. | `TIBTRE_CANACCEPTTIBERIUM_REJECTION_GATES_GHIDRA_REPORT.md`, `PLACETIBERIUM_SPREAD_GERMINATION_CONSTRAINTS_AND_OVERLAY_FRAME_GHIDRA_REPORT.md` | confirmed | yes | IMPLEMENTATION_SAFE |
| New TIBTRE-spawned ore is density/data 3, random flat Riparius variant, and enters growth queue. | `TIBTRE_PLACETIBERIUM_DENSITY_OVERLAY_QUEUE_EFFECTS_GHIDRA_REPORT.md` | confirmed | yes | IMPLEMENTATION_SAFE |
| Stock `Immune=yes` blocks normal terrain damage, but terrain Limbo/destructor paths still exist. | `TIBTRE_TERRAIN_OBJECT_LIFECYCLE_AND_SEEDING_GHIDRA_REPORT.md`, `TERRAIN_CLASS_GHIDRA_REPORT.md` | confirmed | conditional | IMPLEMENTATION_SAFE for stock immunity; NEEDS_REINVESTIGATE for all deletion triggers |
| Terrain occupation bits are type/theater-specific and affect `Cell+0x124` bits 0x04/0x08/0x10. | `TERRAIN_CLASS_GHIDRA_REPORT.md`, `TIBTRE_TERRAIN_OBJECT_LIFECYCLE_AND_SEEDING_GHIDRA_REPORT.md` | confirmed | yes | IMPLEMENTATION_SAFE |
| Scenario `[Terrain]` map write stores only live cell/type, not animation progress. | `TIBTRE_TERRAIN_OBJECT_LIFECYCLE_AND_SEEDING_GHIDRA_REPORT.md`, `0x0071CB90` | confirmed | yes for scenario write | IMPLEMENTATION_SAFE |
| Binary savegame serialization of TerrainClass animation counters is fully known. | lifecycle report deferred item | unknown | yes/conditional | NEEDS_REINVESTIGATE |

## Current Model

GameMD loads overlays before terrain. Then `[Terrain]` entries construct `TerrainClass` instances, immediately `Unlimbo` them at cell center, update terrain/cell side effects, and clear source-cell overlays whose overlay type has the tiberium byte set. Stock TIBTRE01-03 have `SpawnsTiberium=yes`, `IsAnimated=yes`, `AnimationRate=3`, `AnimationProbability=.003`, and `Immune=yes`.

Each live TIBTRE ticks through `TerrainClass::AI`. While idle, it rolls once per AI tick. On success it starts the terrain animation. While active, it does not roll probability. On the midpoint frame, it resets animation state and calls `CellClass::SpreadTiberium(force=true)` from the terrain object's current cell.

For stock map-loaded TIBTREs, the source cell normally has no tiberium overlay because `Unlimbo` cleared it, so forced spread defaults to tiberium type 0/Riparius. If an unusual editor/save/mod path places a recognized source overlay after `Unlimbo`, source-overlay type propagation is real, but that is not the normal stock path.

The spawn target is chosen by random adjacent direction plus wrapped scan over 8 neighbors. The target must pass native tiberium placement gates. New ore is placed as density/data 3, with a random flat overlay variant from the selected tiberium type, and the new cell enters the growth queue.

## Current Rust

Rust currently parses only the terrain object fields needed for spawning in `src/rules/terrain_object_type.rs`: `SpawnsTiberium`, `IsAnimated`, `AnimationRate`, and `AnimationProbability`.

`src/sim/terrain_spawn.rs` now has a stateful delayed model: `TerrainSpawnerState` stores probability, rate, frame count, midpoint, and an `Idle`/`Active` phase. `tick_terrain_spawners_stateful` rolls while idle, delays spawn to midpoint, places density-3 ore, chooses stock flat Riparius variants when registry data is available, and enqueues the growth queue.

`src/app_init.rs` seeds resource nodes from overlays, seeds terrain spawners from map terrain objects, builds the overlay grid, then clears same-cell tiberium overlay/resource state for all `SpawnsTiberium` terrain cells. This approximates `Unlimbo` source-cell clearing for the relevant stock parity case.

`src/sim/world/mod.rs` runs ore growth first and then TIBTRE spawning in phase 7, with resolved terrain, overlay registry, live object, occupancy, and `tiberium_spawning_terrain_cells` context.

## Rust vs GameMD Lifecycle

The important divergence is object ownership. GameMD's spawner is a live `TerrainClass` object; Rust's spawner is a `BTreeMap<(u16,u16), TerrainSpawnerState>` in `ProductionState`.

For stock maps where TIBTREs are never removed, the Rust stateful spawner is close to the player-visible spawn model. For full parity, it is still not a real `TerrainClass` lifecycle: removal/limbo, damage, occupation mark/unmark, and scenario `[Terrain]` write semantics are not represented as one live terrain-object surface.

The existing Rust comment that a spawner "isn't destroyable" is too strong. Stock TIBTRE ordinary damage is blocked by `Immune=yes`, but GameMD has live terrain removal/limbo paths, and mods or non-damage deletion paths can matter later.

## What Is Lacking

1. A proper sim-owned live terrain object model: type id, cell, animation fields, health/liveness/limbo state, occupation bits, and map-save identity.
2. TerrainType parsing beyond spawner fields: `Immune`, `Armor`, `Strength` with `TreeStrength` fallback, `LegalTarget`, `Insignificant`, `RadarInvisible`, `WaterBound`, `IsVeinhole`, `TemperateOccupationBits`, `SnowOccupationBits`, and art `Foundation`.
3. Exact terrain occupation/foundation application to cell state and path/build/passability surfaces.
4. Terrain damage dispatch: Wood warhead gate, `Immune` gate, terrain HP subtract, TIBTRE destruction branch, non-TIBTRE one-shot destruction animation, and limbo/uninit side effects.
5. Synchronization between terrain object lifecycle and `terrain_spawners`: if the terrain object is removed/limboed, its spawner must stop.
6. Scenario map export/write parity for `[Terrain]`: write live non-limbo terrain cell/type only, not Rust snapshot spawner runtime state.
7. Binary savegame TerrainClass serialization remains unresolved; do not infer it from scenario map write.
8. Exact standard trigger/script pathways that can delete or mutate TIBTRE despite `Immune=yes` remain unenumerated.

## What To Implement Next

1. Replace "spawner exists forever" assumptions with a minimal live terrain object lifecycle surface. Keep `terrain_spawners` only as a derived/indexed acceleration synchronized from live terrain objects.
2. Extend `TerrainObjectType` parsing with lifecycle fields: `Immune`, `Armor`, `Strength`/`TreeStrength`, `LegalTarget`, `Insignificant`, `RadarInvisible`, occupation masks, `WaterBound`, `IsVeinhole`; merge art `Foundation`.
3. Apply terrain placement side effects from the live object path: source-cell tiberium overlay/resource clear, occupation mask mark, and target-cell `SpawnsTiberium` rejection data.
4. Add removal/limbo API for terrain objects and make it remove/disable the corresponding spawner, unmark occupation, clear/update cell state, and dirty terrain/radar-owned surfaces through existing sim-side metadata.
5. Add terrain damage only after the object model exists: Wood warhead + `!Immune` gate first, then HP/destruction semantics.
6. Add focused tests:
   - same-cell map ore under TIBTRE is cleared before first tick;
   - stock TIBTRE source without overlay spawns Riparius density 3 after midpoint;
   - target rejects another `SpawnsTiberium` terrain object cell;
   - stock `Immune=yes` damage does not remove TIBTRE;
   - explicit limbo/removal stops future spawns;
   - TIBTRE03 occupation mask differs between temperate and snow.

## Do Not Implement Yet

- Do not implement source-overlay type propagation before preserving `Unlimbo` clearing; otherwise stock maps with same-cell ore will diverge.
- Do not hardcode all TIBTREs as one generic blocker; TIBTRE03 has theater-specific occupation masks.
- Do not use Rust snapshot behavior as GameMD scenario `[Terrain]` save behavior.
- Do not implement `IsFlammable` as a live terrain behavior; the terrain report marks that key as parsed-only/dead for YR.

## Source Ledger

- `TIBTRE_TERRAIN_OBJECT_LIFECYCLE_AND_SEEDING_GHIDRA_REPORT.md`
- `TIBTRE_SOURCE_OVERLAY_TYPE_REACHABILITY_AFTER_UNLIMBO_GHIDRA_REPORT.md`
- `TERRAIN_CLASS_GHIDRA_REPORT.md`
- `TIBTRE_TERRAINCLASS_AI_TIMING_AND_RNG_GHIDRA_REPORT.md`
- `TIBTRE_RETAIL_SHP_FRAME_COUNTS_AND_MIDPOINT_TICKS_GHIDRA_REPORT.md`
- `TIBTRE_SPREADTIBERIUM_FORCE_TYPE_AND_FLAG_GATE_GHIDRA_REPORT.md`
- `TIBTRE_CANACCEPTTIBERIUM_REJECTION_GATES_GHIDRA_REPORT.md`
- `TIBTRE_PLACETIBERIUM_DENSITY_OVERLAY_QUEUE_EFFECTS_GHIDRA_REPORT.md`
- `PLACETIBERIUM_SPREAD_GERMINATION_CONSTRAINTS_AND_OVERLAY_FRAME_GHIDRA_REPORT.md`
- Ghidra spot-checks this synthesis: `TerrainClass::AI @ 0x0071C730`, `TerrainClass::Unlimbo @ 0x0071D000`.
- Rust inspected: `src/sim/terrain_spawn.rs`, `src/app_init.rs`, `src/sim/world/mod.rs`, `src/sim/production/production_types.rs`, `src/rules/terrain_object_type.rs`.
