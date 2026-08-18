# Static Terrain Light Keys Non-Emitter Trace

Date: 2026-05-24

Scenario: Place stock standard YR terrain objects `TIBTRE01`, `TIBTRE02`, `TIBTRE03`, and one lamp-looking `LT_*` terrain object (`LT_GEN01`) on a flat map with no building lights and no radiation lights. Verify that terrain `Light*` keys do not create emitted point-light radius.

Scope: terrain-object light-source ownership only. This trace does not judge full terrain sprite palette parity, TIBTRE ore spawning, terrain animation timing, building lamps, radiation, combat flashes, spark lights, or superweapon lighting.

## Verdict

PASS: 4 | FAIL: 0 | UNCHECKED: 1 | NOT-IMPLEMENTED: 0

Overall status: COMPLETE.

## Pipeline

`rulesmd.ini/artmd.ini terrain entries -> gamemd TerrainTypeClass reader / Rust TerrainObjectType reader -> map [Terrain] placement -> terrain lifecycle/source collection -> rendered terrain object consumes existing cell lighting`

## Concrete Data

Chosen stock objects:

- `TIBTRE01`, `TIBTRE02`, `TIBTRE03`: each has `LightVisibility=4000`, `LightIntensity=0.01`, `LightRedTint=0.01`, `LightGreenTint=1.5`, `LightBlueTint=0.01` in `ini/rulesmd.ini`.
- `LT_GEN01`: lamp-looking terrain object with no `Light*` keys in `ini/rulesmd.ini`; `Foundation=1x1` in `ini/artmd.ini`.
- Scenario source counts: 4 terrain objects, 0 structure entities, 0 radiation sites.

## Stage Results

### Stage 1 - Terrain INI presence

gamemd input:

- TIBTRE objects have 5 `Light*` lines each.
- `LT_GEN01` has 0 `Light*` lines.

Rust input:

- Same repo INI data is read.
- `TerrainObjectType::from_ini_section` reads `SpawnsTiberium`, `IsAnimated`, `AnimationRate`, and `AnimationProbability` only.

Computed equality:

- TIBTRE sections with nonzero `LightIntensity`: gamemd source data 3, Rust source data 3.
- `LT_GEN01` terrain light-key count: gamemd source data 0, Rust source data 0.

Verdict: PASS for input-data equality.

### Stage 2 - Terrain type parser light ownership

gamemd:

- `TerrainTypeClass__ReadINI_Full @ 0x0071DEA0` was spot-checked read-only in Ghidra.
- It calls `ObjectTypeClass__ReadINI`, then reads terrain fields including `IsVeinhole`, `WaterBound`, `SpawnsTiberium`, `IsFlammable`, `Foundation`, `RadarColor`, `IsAnimated`, `AnimationRate`, `AnimationProbability`, `TemperateOccupationBits`, and `SnowOccupationBits`.
- It has 0 reads of `LightVisibility`, `LightIntensity`, `LightRedTint`, `LightGreenTint`, or `LightBlueTint`.

Rust:

- `src/rules/terrain_object_type.rs:16` defines no light fields.
- `src/rules/terrain_object_type.rs:34` reads no light keys.

Computed equality:

- terrain-owned emitted-light fields stored per type: gamemd 0, Rust 0.

Verdict: PASS.

### Stage 3 - Terrain placement creates no LightSource

gamemd:

- `TerrainClass__Unlimbo @ 0x0071D000` was spot-checked read-only in Ghidra.
- Its callee set is `ObjectClass__Reveal`, `MapClass__Get_CellClass`, `CellClass__Get_Cell_At`, and `TacticalClass__CoordsToClient2`.
- It does not call `LightSourceClass__Constructor`.

Rust:

- Map `[Terrain]` creates `TerrainObject` records at `src/map/overlay.rs:166`.
- No terrain placement path creates `PointLight`.

Computed equality for four placed terrain objects:

- terrain LightSource/PointLight objects created at placement: gamemd 0, Rust 0.

Verdict: PASS.

### Stage 4 - LightSource caller/source collection census

gamemd:

- `LightSourceClass__Constructor @ 0x00554760` caller census was spot-checked read-only.
- Verified callers: `BuildingClass__OnConstructionComplete @ 0x00445F80`, `BuildingClass__Unlimbo @ 0x00440580`, and `RadSiteClass__Activate @ 0x0065B580`.
- Terrain callers: 0.

Rust:

- `collect_live_building_lights` filters `EntityCategory::Structure` at `src/app_init.rs:193`.
- `collect_building_lights` filters `EntityCategory::Structure` at `src/map/lighting.rs:395`.
- The scoped scenario has 0 structure entities and only terrain objects.

Computed equality:

- emitted point lights attributable to terrain objects: gamemd 0, Rust 0.

Verdict: PASS.

### Stage 5 - Screen-visible emitted radius

gamemd:

- With no terrain-created `LightSourceClass`, no emitted point-light radius exists around `TIBTRE01/02/03` or `LT_GEN01`.
- `TerrainClass__Draw_It @ 0x0071C1B0` consumes existing cell lighting via cell light fields and passes a scalar to `CC_Draw_Shape`; it does not create a source.

Rust:

- Terrain objects consume `state.lighting_grid.terrain_object_tint_at((obj.rx, obj.ry))` at `src/app_instances/overlays.rs:433`.
- `terrain_object_tint_at` returns the existing cell tint at `src/map/lighting.rs:245`.
- No source is added for terrain objects, so no terrain-origin radius expands from the four placed objects.

Computed equality:

- emitted point-light radius from the four terrain objects: gamemd 0 cells, Rust 0 cells.

Verdict: PASS for non-emission.

Unchecked:

- Exact terrain sprite palette/scalar equality on a flat no-light map was not computed numerically against gamemd. This belongs to broader static cell-light / LightConvert parity, not terrain source ownership.

## Player-Visible Findings

No FAIL or NOT-IMPLEMENTED findings in this scoped trace.

The player-visible parity requirement is negative: ore trees and lamp-looking `LT_*` terrain scenery must not create a green glow or lamp radius by themselves. Current Rust matches that source-ownership behavior.

## Adjacent Findings

- Existing `TERRAIN_CLASS_GHIDRA_REPORT.md` wording is stale where it says TIBTRE light keys are consumed by a separate light-source parser. The newer terrain-light ownership report resolves this: no terrain parser consumes them for emitters in standard YR.
- Terrain objects should continue to consume resolved cell lighting when drawn. Removing terrain tint consumption would be a separate regression.
- Building lamp sections such as `GALITE`, `INGALITE`, and colored/invisible lamp buildings are separate structure-owned light sources and are outside this trace.

## Sources

- `C:/Users/enok/Documents/ra2-rust-game-docs/TERRAIN_OBJECT_LIGHT_KEYS_AND_LIGHTSOURCE_OWNERSHIP_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/TERRAIN_CLASS_GHIDRA_REPORT.md`
- `C:/Users/enok/Documents/ra2-rust-game-docs/LIGHTSOURCE_LIFECYCLE_POWER_DAMAGE_SAVELOAD_GHIDRA_REPORT.md`
- `ini/rulesmd.ini`
- `ini/artmd.ini`
- `src/rules/terrain_object_type.rs`
- `src/map/lighting.rs`
- `src/app_init.rs`
- `src/app_instances/overlays.rs`
- Read-only Ghidra spot checks: `0x0071DEA0`, `0x0071D000`, `0x0071C1B0`, `0x00554760`, and `LightSourceClass__Constructor` caller census.
