# TIBTRE Nonimmune Damage Removes Spawner Trace - 2026-05-27

## Scenario

Concrete modded setup only:

- `[TIBTRE01] Immune=no`
- `[TIBTRE01] Strength=10`
- `[TIBTRE01] SpawnsTiberium=yes`
- `[TIBTRE01] IsAnimated=yes`
- A `Wood=yes` warhead applies 10 base damage to the TIBTRE01 source cell.
- Warhead `Verses[wood]=100%` for this trace, so effective terrain damage is exactly 10.

## Verdict Summary

PASS: 7 | FAIL: 2 | UNCHECKED: 2 | NOT-IMPLEMENTED: 0

Current Rust passes the core lifecycle requirement for this exact scenario: the nonimmune TIBTRE reaches 0 HP, is marked destroyed, and the derived spawner/tiberium-source/occupation indices are removed. The player-visible mismatch is that GameMD's TIBTRE kill branch also creates the terrain explosion animation and applies chained radius-1 `C4Warhead` area damage before uninitializing the terrain object; current Rust removes the terrain object without those chained effects.

## Pipeline

`Wood=yes impact on source cell` -> `terrain damage dispatch` -> `Wood && !Immune gate` -> `damage * Verses[wood]` -> `HP 10 -> 0` -> `TIBTRE death branch` -> `remove live TerrainClass / derived Rust spawner state` -> `cell passability/radar/visual consequences`

## Evidence

- GameMD active YR `TerrainClass::Take_Damage @ 0x0071B920`: returns 0 if warhead is null; otherwise requires `Warhead+0x147 Wood != 0` and `TerrainTypeClass+0x233 Immune == 0`; calls `ObjectClass::ReceiveDamage`; when return code is 4 and `TerrainTypeClass+0x2B1 SpawnsTiberium != 0`, it creates an explosion anim, calls `Apply_area_damage(..., RulesClass+0xFA8, radius=1, ...)`, then calls `vtable+0xF8` (`ObjectClass::UnInit`).
- GameMD active YR `ObjectClass::ReceiveDamage @ 0x005F5390`: reads/writes health at object `+0x6C`, computes damage from type armor and warhead verses when the force flag is not set, subtracts health, clamps lethal damage to current health, and returns code 4 when health becomes zero.
- GameMD active YR `TerrainClass::Limbo @ 0x0071C930`: if not already limboed, decrements 8-neighbor terrain counters, clears source-cell `CellClass+0x124` bit `0x40`, conceals the object, recalculates the cell, assigns orphaned zones, refreshes zones, and marks radar terrain dirty outside map-editor mode.
- GameMD active YR `TerrainClass::Mark_Occupation @ 0x0071C110` / `Unmark_Occupation @ 0x0071C070`: source mask bits `1/2/4` map to `CellClass+0x124` bits `0x04/0x08/0x10`; snow uses `TerrainTypeClass+0x2AC`, otherwise `+0x2A8`.
- Research anchors: `docs/research/TERRAIN_CLASS_GHIDRA_REPORT.md` sections 13.6-13.10 and 15.5; `docs/research/TIBTRE_TERRAIN_OBJECT_LIFECYCLE_AND_SEEDING_GHIDRA_REPORT.md` sections 4-5 and 10.
- Rust source: `src/sim/terrain_object.rs`, `src/sim/terrain_spawn.rs`, `src/sim/combat/mod.rs`, `src/sim/world/mod.rs`, `src/rules/terrain_object_type.rs`, `src/rules/warhead_type.rs`.

## Stage Results

| Stage | Rust result | GameMD result | Verdict |
|---|---:|---:|---|
| Terrain type data | `Strength=10`, `Immune=no`, default `Armor=wood`, default occupation mask `7` through parser/test fixture. | Type fields are active; `Strength` feeds max HP, `Immune` gates damage, default armor is Wood unless overridden, default occupation mask is 7. | PASS |
| Warhead data | `Wood=yes`; test fixture has `Verses[wood]=100%`. | `Warhead+0x147 Wood` is the hard TerrainClass damage gate; `ObjectClass::ReceiveDamage` applies verses to armor. | PASS |
| Dispatch into terrain damage | Current whole-combat Rust emits one `TerrainDamageEvent` for the impact cell when `warhead.wood && base_damage > 0`; direct helper call receives `(10,11), 10, WH`. | GameMD area/single-cell object dispatch calls terrain vtable slot `+0x16C` for objects in the affected cell. | PASS for this exact source-cell hit; UNCHECKED for every AoE offset ordering outside this scenario. |
| Wood/immune gate | `damage_terrain_object_at_cell` returns ignored unless `base_damage > 0`, `warhead.wood`, live object, and `!terrain_type.immune`. | `TerrainClass::Take_Damage` requires non-null warhead, `Warhead.Wood`, and `!TerrainType.Immune` before `ReceiveDamage`. | PASS |
| Damage math | `10 * 100 / 100 = 10`; health `10 - 10 = 0`. | Terrain RTTI does not receive BuildingClass min-damage clamp; with `Verses[wood]=100%`, damage is 10; health `10 -> 0`; `ReceiveDamage` return code 4. | PASS |
| TIBTRE destruction branch | Rust returns `TerrainDamageResult::Destroyed`, limbos the object, then marks lifecycle `Destroyed`. | `Take_Damage` sees return code 4 and `SpawnsTiberium`, creates an explosion anim, applies radius-1 `RulesClass+0xFA8` area damage, then calls `ObjectClass::UnInit`. | FAIL: Rust omits explosion anim and chained AoE before removal. |
| Source-cell object index | Rust removes `terrain_object_cells[(10,11)]`. | `ObjectClass::UnInit` calls Limbo and queues delete; TerrainClass destructor/removal path removes the terrain object from the global terrain object array. | PASS at lifecycle level; UNCHECKED for exact same-tick global-array delete timing. |
| Derived spawner indices | Rust removes `terrain_spawners[(10,11)]` and `tiberium_spawning_terrain_cells[(10,11)]`. | GameMD has no separate owner spawner map; spawning is owned by live `TerrainClass::AI`, and UnInit/Limbo/deletion means no live source object should keep producing TIBTRE AI spawns. | PASS for required derived-index behavior. |
| Occupation/source-cell blocking | Rust removes `terrain_occupation_bits[(10,11)]` and clears resolved terrain object block when a grid is supplied. | GameMD `Unmark_Occupation` clears occupation bits `0x04/0x08/0x10`; `Limbo` clears source bit `0x40`, decrements neighbor counters, and recalculates cell/radar/zone state. | FAIL: Rust removes its local occupation index, but does not model `Cell+0x40`, 8-neighbor `+0x122`, or exact zone/radar dirty sequence. |
| Dirty/path cache consequence | Rust sets `destroyed_structure=true` on `Destroyed`, causing broader path/cache update behavior in `World`. | GameMD dirty sequence includes tactical rect dirtying, object mark/update call, Limbo cell recalculation, zone assignment/refresh, and radar terrain dirty. | PASS only for "some path/cache update requested"; exact dirty ordering is UNCHECKED. |
| Focused test | `cargo test -q nonimmune_tibtree_death_limbos_object_and_removes_spawner_indices --lib` passed. | No direct runtime harness output was captured from GameMD for this exact modded INI; comparison is from read-only decompile and verified research docs. | PASS as Rust regression coverage, not a standalone binary parity proof. |

## Top Findings

1. TIBTRE death branch side effects are incomplete: current Rust removes the object/spawner but skips GameMD's explosion animation and radius-1 `C4Warhead` chained AoE. Player-visible difference: nearby objects will not take the tree-death splash and the visual explosion is absent.
2. Limbo cell side effects are approximated: current Rust removes its own occupation index and resolved terrain block, but does not model GameMD's source `Cell+0x124` bit `0x40`, 8-neighbor `Cell+0x122` decrement, exact `RecalcAttributes`, zone refresh, and radar dirty ordering. Player-visible difference can appear in movement/buildability/radar refresh edge cases after terrain destruction.
3. Exact GameMD terrain-object global-array deletion timing after `ObjectClass::UnInit` was not measured with a live runtime harness. The decompiled path proves UnInit/Limbo and queued deletion; exact same-tick scheduler removal remains UNCHECKED.
4. Whole-AoE terrain object iteration is outside this concrete source-cell scenario. Current combat only emits terrain damage for the impact cell in the inspected path; equivalence for every `CellSpread` offset and GameMD per-cell/per-object ordering remains UNCHECKED here.

## Adjacent Findings

- Stock TIBTRE01-03 remain `Immune=yes`; this trace intentionally uses the modded `Immune=no` scenario and does not change the stock immunity verdict.
- Non-TIBTRE tree death is different from TIBTRE death: GameMD non-TIBTRE trees enter a short destroy-animation path before UnInit; TIBTRE uses the explosion/AoE branch and then UnInit.
- `Catch_Fire` remains TS/dead for active YR terrain fire propagation per existing research; this trace only covers direct `Wood=yes` damage.

## Return Contract Snapshot

PASS: 7 | FAIL: 2 | UNCHECKED: 2 | NOT-IMPLEMENTED: 0

Status: COMPLETE
