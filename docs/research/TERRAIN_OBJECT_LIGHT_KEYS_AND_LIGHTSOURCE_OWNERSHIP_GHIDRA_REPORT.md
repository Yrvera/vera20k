# Terrain Object Light Keys And LightSource Ownership - Ghidra Research Report

**Address(es):** `0x0071DEA0`, `0x0071DA80`, `0x0071D000`, `0x0071C930`, `0x0071C730`, `0x0071C1B0`, `0x0045FE50`, `0x00440580`, `0x00554760`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** Whether standard YR terrain object types or other non-building rules sections with `LightVisibility`, `LightIntensity`, and `Light*Tint` create or own `LightSourceClass` data. This directly resolves LRD-14 from `LIGHT_RULES_ART_PARSER_DEFAULTS_GHIDRA_REPORT.md`.  
**Non-Scope:** Building lamp parser constants, ordinary map `[Lighting]`, LightSource falloff, LightConvert palette tables, radiation light behavior beyond caller classification, combat/particle lights, and `BuildingLightClass` spotlights.  
**Confidence:** High for scoped terrain parser ownership and no terrain LightSource creation.  
**Active in YR:** Yes for terrain type parsing, terrain object placement/draw/tick paths, and building/radiation LightSource callers. Terrain-owned light creation: No.

## Target Question

Do terrain object types or non-building rules sections that contain `LightVisibility`, `LightIntensity`, `LightRedTint`, `LightGreenTint`, or `LightBlueTint` create `LightSourceClass` data in standard Yuri's Revenge? If not, which parser owns those keys and how should Rust treat stock terrain/lightpost-looking entries?

## Non-Goals

- Do not implement or modify Rust.
- Do not mutate Ghidra state.
- Do not re-investigate building light read constants, map ambience, or ordinary LightSource falloff.
- Do not treat terrain object draw consumption of cell lighting as proof that terrain objects emit light.

## Evidence Needed To Mark COMPLETE

- Verify the only binary string owners for all five `Light*` keys.
- Decompile `TerrainTypeClass` constructor and reader to identify parsed fields/defaults.
- Decompile live `TerrainClass` placement, removal, tick, and draw paths for LightSource calls or fields.
- Verify `LightSourceClass__Constructor` callers in the binary call graph.
- Check stock INI placement of the suspicious light keys, especially `TIBTRE01..03` and `LT_*`.
- Scan current Rust terrain/light collection surfaces and name the delta.

## Stop Conditions

- Stop once terrain ownership and LightSource caller census are resolved.
- Stop before patching stale docs or Rust; provide replacement wording only.
- Stop before global render parity for terrain objects beyond source-vs-consumer classification.

## 1. Overview

Terrain object type parsing in standard YR does not own the five building lamp keys. `TerrainTypeClass__ReadINI_Full @ 0x0071DEA0` reads terrain fields such as `SpawnsTiberium`, `IsFlammable`, `IsAnimated`, `AnimationRate`, `AnimationProbability`, `RadarColor`, and occupation bits, but it does not reference `LightVisibility`, `LightIntensity`, or any `Light*Tint` key.

The five `Light*` key strings have one documented binary owner: `BuildingTypeClass_ReadINI_Water @ 0x0045FE50`. Stock `TIBTRE01..03` contain light-key lines in `rulesmd.ini`, but standard `gamemd.exe` ignores them for terrain objects. Stock `LT_GEN*`, `LT_SGN*`, and `LT_EUR*` terrain object sections also do not contain light keys; their names/art look like light posts, but they are ordinary terrain sprites.

`LightSourceClass__Constructor @ 0x00554760` has three verified callers in this binary: `BuildingClass__Unlimbo`, `BuildingClass__OnConstructionComplete`, and `RadSiteClass__Activate`. No `TerrainClass` or `TerrainTypeClass` caller constructs, enables, disables, updates, stores, saves, or loads a terrain-owned LightSource.

## 2. Class Layout / Key Offsets

### TerrainTypeClass fields in this slice

| Offset | Key / field | Default | Reader | Active in YR | Notes |
|---:|---|---:|---|---|---|
| `+0x298` | `Foundation` | `0` | `0x0071DEA0` via `FUN_00474DA0` | Yes | Art-style foundation selector. |
| `+0x29C..0x29E` | `RadarColor` | zero unless derived | `0x0071DEA0` | Yes | If type is tiberium-like, default can be derived before explicit read. |
| `+0x2A0` | `AnimationRate` | `0` | `0x0071DEA0` | Yes | Used by `TerrainClass__AI`. |
| `+0x2A4` | `AnimationProbability` | `0.0` | `0x0071DEA0` | Yes | Float read via `CCINIClass__ReadDouble`; used by `TerrainClass__AI`. |
| `+0x2A8` | `TemperateOccupationBits` | `7` | `0x0071DEA0` | Yes | Affects occupied cell bits in other terrain paths. |
| `+0x2AC` | `SnowOccupationBits` | `7` | `0x0071DEA0` | Yes | Snow variant of occupation mask. |
| `+0x2B0` | `WaterBound` | `false` | `0x0071DEA0` | Yes | Read using shared `WaterBound` string. |
| `+0x2B1` | `SpawnsTiberium` | `false` | `0x0071DEA0` | Yes | TIBTRE ore-spawn gate. |
| `+0x2B2` | `IsFlammable` | `false` | `0x0071DEA0` | Parsed, no YR runtime consumer per prior report | TS legacy field. |
| `+0x2B3` | `IsAnimated` | `false` | `0x0071DEA0` | Yes | TIBTRE animation and draw-frame gate. |
| none | `LightVisibility` | none | not read by terrain reader | No as terrain key | String owner is building type parser only. |
| none | `LightIntensity` | none | not read by terrain reader | No as terrain key | String owner is building type parser only. |
| none | `LightRedTint/LightGreenTint/LightBlueTint` | none | not read by terrain reader | No as terrain keys | String owner is building type parser only. |

### TerrainClass instance fields checked for source ownership

`TerrainClass` instance size is documented as `0xE0` in `TERRAIN_CLASS_GHIDRA_REPORT.md`; checked live paths here use terrain instance fields through `+0x9C..+0xCD` and type pointer `+0xC8`. No field analogous to building `+0x614` appears in constructor, Unlimbo, Limbo, AI, or Draw_It. No TerrainClass path calls `LightSourceClass__Constructor`, `0x00554A60`, `0x00554A80`, or `0x00554AA0`.

## 3. Core Logic

### TerrainType parser ownership

`TerrainTypeClass__Constructor @ 0x0071DA80` initializes terrain-specific defaults:

- `+0x2A8 = 7`, `+0x2AC = 7` for occupation masks.
- `+0x2A0 = 0`, `+0x2A4 = 0` for animation timing/probability.
- `+0x2B0..+0x2B4 = 0` for `WaterBound`, `SpawnsTiberium`, `IsFlammable`, `IsAnimated`, and `IsVeinhole`.
- No light visibility, intensity, tint, or LightSource pointer fields are initialized.

`TerrainTypeClass__ReadINI_Full @ 0x0071DEA0` first calls `ObjectTypeClass__ReadINI`, then reads only the terrain-specific keys listed above. The Ghidra string anchor report and decompile show `SpawnsTiberium @ 0x00844674`, `IsFlammable @ 0x00844668`, `IsAnimated @ 0x0084465C`, `TemperateOccupationBits @ 0x0084461C`, and `SnowOccupationBits @ 0x00844608` in this function. No `LightVisibility`, `LightIntensity`, or tint string is referenced by this reader.

### Light key string ownership

Ghidra string anchor search for `Light` reports:

- `LightVisibility @ 0x0081A92C` -> documented owner `BuildingTypeClass_ReadINI_Water`.
- `LightIntensity @ 0x0081A91C` -> documented owner `BuildingTypeClass_ReadINI_Water`.
- `LightRedTint @ 0x0081A90C` -> documented owner `BuildingTypeClass_ReadINI_Water`.
- `LightGreenTint @ 0x0081A8FC` -> documented owner `BuildingTypeClass_ReadINI_Water`.
- `LightBlueTint @ 0x0081A8EC` -> documented owner `BuildingTypeClass_ReadINI_Water`.

No terrain reader appears in those string xrefs. Active in YR: Yes for building types; No for terrain types.

### Terrain lifecycle paths checked

`TerrainClass__Unlimbo @ 0x0071D000`:

- Calls `ObjectClass__Reveal`.
- Increments neighbor cell `+0x122` around the terrain cell.
- Updates tactical render extent.
- Clears incompatible overlay from the object cell when overlay type flag `+0x2A9` is set.
- Callees are `ObjectClass__Reveal`, `MapClass__Get_CellClass`, `CellClass__Get_Cell_At`, and `TacticalClass__CoordsToClient2`.
- No LightSource constructor, enable, disable, or update call.

`TerrainClass__Limbo @ 0x0071C930`:

- Decrements neighbor cell `+0x122`.
- Clears cell flag bit `0x40` at `Cell+0x124`.
- Calls `ObjectClass__Conceal`, `CellClass__RecalcAttributes`, `MapClass__AssignOrphanedCellZone`, `FUN_00584550`, and `RadarClass__MarkTerrainDirty`.
- No LightSource teardown path exists.

`TerrainClass__AI @ 0x0071C730`:

- Runs `ObjectClass__AI`.
- If `type+0x2B3 IsAnimated` and no animation is active, rolls `Random__Next() % 1000000` against `AnimationProbability`.
- Advances the animation timer when the CD timer expires.
- If `type+0x2B1 SpawnsTiberium` and `type+0x2B3 IsAnimated`, calls `CellClass__SpreadTiberium(1)` at the SHP midpoint.
- Callees are `ObjectClass__AI`, `Random__Next`, `CDTimerClass__GetTimeRemaining`, `CellClass__Get_Cell_At`, and `CellClass__SpreadTiberium`.
- No LightSource update or source-position path exists.

### LightSource caller census

`get_function_callers(LightSourceClass__Constructor)` returned exactly:

- `BuildingClass__Unlimbo @ 0x00440580`.
- `BuildingClass__OnConstructionComplete @ 0x00445F80`.
- `RadSiteClass__Activate @ 0x0065B580`.

This matches the prior LightSource lifecycle reports. No terrain caller is present. Active in YR: Yes for building lamps and radiation; No for terrain-owned LightSource.

### Terrain objects are lighting consumers, not source owners

`TerrainClass__Draw_It @ 0x0071C1B0` is a standard YR draw consumer of cell lighting:

- Gets the terrain object's cell and checks `Cell+0x34`.
- If `Cell+0x34` is null, calls `FUN_00483E30(0, 0x10000, 0, 1000, 1000, 1000)` to lazy-init a light profile.
- Uses `(short)Cell+0x10C` for normal terrain object sprite draw.
- Uses `(short)Cell+0x10A` and shifts Y by `-0x10` for `SpawnsTiberium` terrain.
- Calls `CC_Draw_Shape` with the selected scalar light value.

Active in YR: Yes. This proves terrain objects are affected by already-computed cell lighting. It does not create a terrain-owned LightSource.

## 4. INI Keys

| Key / section | Stock examples | Binary owner | Effect in standard YR | Active in YR |
|---|---|---|---|---|
| `[TerrainTypes]` registry | `TIBTRE01..03`, `LT_GEN01..04`, `LT_SGN01..04`, `LT_EUR01..02`, `POLE01..02`, `SIGN*`, `TRFF*`, `SPKR01` | `RulesClass__Process` -> `TerrainTypeClass__Find_Or_Allocate` -> terrain type read | Creates TerrainTypeClass definitions only. | Yes |
| `SpawnsTiberium` | `TIBTRE01..03=yes` | `TerrainTypeClass__ReadINI_Full @ 0x0071DEA0` | Gates TIBTRE ore spawn with `IsAnimated`. | Yes |
| `IsAnimated` | `TIBTRE01..03=yes` | `TerrainTypeClass__ReadINI_Full @ 0x0071DEA0` | Gates animation and TIBTRE spawn. | Yes |
| `AnimationRate`, `AnimationProbability` | `3`, `.003` on TIBTRE | `TerrainTypeClass__ReadINI_Full @ 0x0071DEA0` | TIBTRE animation/spawn timing. | Yes |
| `LightVisibility`, `LightIntensity`, `Light*Tint` on `TIBTRE01..03` | `4000`, `0.01`, green tint lines in `rulesmd.ini` | No terrain owner; strings owned by building type parser | Ignored for terrain objects; no Terrain LightSource is created. | No as terrain keys |
| `LT_GEN*` terrain object sections | Names say lightpost, no light keys present | `TerrainTypeClass__ReadINI_Full` for ordinary terrain fields | Static terrain sprites only; cell-light consumers when drawn. | Yes as terrain, No as light source |
| Building lamp sections | `GALITE`, `INGALITE`, `REDLAMP`, etc. | `BuildingTypeClass_ReadINI_Water @ 0x0045FE50` | Building LightSource data, gated by nonzero `LightIntensity`. | Yes |

Stock INI detail: `rulesmd.ini` `TIBTRE01`, `TIBTRE02`, and `TIBTRE03` contain the five light-key lines, but the terrain parser does not read those string keys. Stock terrain `LT_*` sections around the terrain registry contain names, flammability/radar/occupation fields, and art `Foundation=1x1`, but no light-key lines.

## 5. Integration Points

- `RulesClass__Process @ 0x00665DE0` enumerates `[TerrainTypes]`, reads each registry value, and calls `TerrainTypeClass__Find_Or_Allocate`.
- `TerrainTypeClass__ReadINI_Full @ 0x0071DEA0` owns terrain type data and does not call building parser code.
- Map `[Terrain]` entries create `TerrainClass` instances, not `BuildingClass` instances.
- `TerrainClass__Unlimbo`, `TerrainClass__Limbo`, and `TerrainClass__AI` contain no LightSource lifecycle calls.
- `TerrainClass__Draw_It` consumes existing `CellClass` lighting fields.
- `LightSourceClass__Constructor @ 0x00554760` callers are building placement/completion and radiation activation only.

## 6. Current Rust Implementation Status

- `src/rules/terrain_object_type.rs` parses only terrain-spawn fields and explicitly omits light fields. For this exact terrain-light ownership question, omitting terrain-owned `LightVisibility` and tint fields is binary-correct.
- `src/rules/object_type.rs` parses building/object light keys for structures; this is the correct owner for stock building lamp sections.
- `src/map/lighting.rs::collect_building_lights` collects only `EntityCategory::Structure` entities and reads `ObjectType` light fields. That is correct for building lamp ownership.
- `src/app_init.rs::rebuild_lighting_grid_from_sim` accumulates live building lights only; no terrain source collection exists. That matches standard YR for terrain objects.
- `src/app_instances/overlays.rs` tints terrain object sprites through `state.lighting_grid.terrain_object_tint_at((obj.rx, obj.ry))`. That is directionally correct because terrain objects are cell-light consumers, but exact parity still depends on the broader cell-light scalar/profile model from prior lighting reports.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `LightVisibility` string ownership | verified | string `0x0081A92C`; owner `BuildingTypeClass_ReadINI_Water` | none |
| `LightIntensity` string ownership | verified | string `0x0081A91C`; owner `BuildingTypeClass_ReadINI_Water` | none |
| `LightRedTint/LightGreenTint/LightBlueTint` ownership | verified | strings `0x0081A90C/0x0081A8FC/0x0081A8EC`; owner `BuildingTypeClass_ReadINI_Water` | none |
| `TerrainTypeClass__Constructor` defaults | verified | decompile `0x0071DA80` | none |
| `TerrainTypeClass__ReadINI_Full` field set | verified | decompile `0x0071DEA0`; callees and string anchors | none |
| `RulesClass__Process` terrain registry path | verified | decompile `0x00665DE0`; `TerrainTypes @ 0x00839DCC` | none |
| Terrain Unlimbo lifecycle | verified | decompile/callees `0x0071D000` | none for light ownership |
| Terrain Limbo lifecycle | verified | decompile/callees `0x0071C930` | none for light ownership |
| Terrain AI tick | verified | decompile/callees `0x0071C730` | none for light ownership |
| Terrain draw consumption of cell light | verified | decompile `0x0071C1B0`; `LIGHTING_DRAW_CONSUMERS_CELL_FIELDS_GHIDRA_REPORT.md` | exact renderer parity belongs to map lighting implementation |
| LightSource constructor caller census | verified | `get_function_callers(LightSourceClass__Constructor)` | none for terrain ownership |
| Stock `TIBTRE01..03` light-key lines | verified | `ini/rulesmd.ini` lines around `28114..28148` | none |
| Stock `LT_*` terrain sections | verified | `ini/rulesmd.ini` terrain registry and sections around `27912..27975`; `ini/artmd.ini` around `12850..12886` | none |
| Current Rust source ownership | verified | `src/rules/terrain_object_type.rs`, `src/map/lighting.rs`, `src/app_init.rs`, `src/app_instances/overlays.rs` | no code changes made |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] TOL-01 - Do terrain type readers reference the five Light* strings? -> No; `TerrainTypeClass__ReadINI_Full` reads terrain keys only, while all five Light* strings are owned by `BuildingTypeClass_ReadINI_Water`.` (evidence: `0x0071DEA0`, string anchors `0x0081A92C..0x0081A8EC`)`
- `[RESOLVED] TOL-02 - Are stock `TIBTRE01..03` Light* lines live? -> No as terrain light keys; they are present in INI but unread by the terrain parser.` (evidence: `ini/rulesmd.ini`, `0x0071DEA0`)`
- `[RESOLVED] TOL-03 - Do `LT_*` terrain lightpost-looking objects create light? -> No; they are TerrainType entries with ordinary terrain fields and no Light* keys in stock rules/art.` (evidence: `ini/rulesmd.ini` registry/sections, `ini/artmd.ini` `Foundation=1x1`, `0x0071DEA0`)`
- `[RESOLVED] TOL-04 - Does TerrainTypeClass initialize hidden light fields? -> No light fields or LightSource pointer defaults are present in checked constructor fields.` (evidence: `0x0071DA80`)`
- `[RESOLVED] TOL-05 - Does TerrainClass placement allocate or enable LightSource? -> No; Unlimbo callees do not include constructor or wrappers.` (evidence: `0x0071D000`, function callees)`
- `[RESOLVED] TOL-06 - Does TerrainClass removal disable/delete LightSource? -> No; Limbo has no LightSource teardown path.` (evidence: `0x0071C930`, function callees)`
- `[RESOLVED] TOL-07 - Does TerrainClass AI update a LightSource position/intensity? -> No; AI only handles object AI, animation timing, and TIBTRE ore spread.` (evidence: `0x0071C730`, function callees)`
- `[RESOLVED] TOL-08 - Who calls `LightSourceClass__Constructor`? -> Building Unlimbo, Building OnConstructionComplete, and RadSite Activate only.` (evidence: Ghidra caller census for `0x00554760`)`
- `[RESOLVED] TOL-09 - Are terrain objects affected by lighting? -> Yes as draw consumers: Draw_It reads cell light fields/profile and passes scalar light to `CC_Draw_Shape`.` (evidence: `0x0071C1B0`)`
- `[RESOLVED] TOL-10 - Does current Rust need a terrain-owned light source collection path? -> No for standard YR; current building-only source collection matches this ownership slice.` (evidence: Rust scan plus `0x00554760` caller census)`
- `[RESOLVED] TOL-11 - Should Rust keep terrain object sprite tinting? -> Yes, but as cell-light consumption, not source emission.` (evidence: `0x0071C1B0`, `src/app_instances/overlays.rs`)`
- `[RESOLVED] TOL-12 - Is LRD-14 now resolved? -> Yes; terrain section light keys are ignored by standard YR terrain parsing and do not create LightSource data.` (evidence: this report)`

## 9. Visual / Render Consumer Ledger

This report does not claim full terrain render parity. It only separates lighting source ownership from terrain object draw consumption.

| Order | Function / address | Condition / flag proof | Asset / frame | Light input | Active for target? | Role |
|---:|---|---|---|---|---|---|
| 1 | `TerrainClass__Draw_It @ 0x0071C1B0` | terrain object exists and image data is available | terrain SHP frame selected by `IsAnimated` / current frame / burning flag | lazy `Cell+0x34`; scalar `Cell+0x10C` or TIBTRE `Cell+0x10A` | Yes | Terrain object cell-light consumer |

## 10. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| Terrain type light-key lines do not create `LightSourceClass` data. | `0x0071DEA0`; Light* string anchors owned by `0x0045FE50`; caller census for `0x00554760` | No source-collection delta needed | `src/rules/terrain_object_type.rs`, `src/map/lighting.rs::collect_building_lights`, `src/app_init.rs::rebuild_lighting_grid_from_sim` | Keep terrain object types out of LightSource collection; do not parse terrain Light* keys for emission in standard YR mode. | A map with only `TIBTRE01` and no building/radiation lights should not create point lights even though TIBTRE has `LightIntensity=0.01` in stock INI. | `terrain_tibtre_light_keys_do_not_emit_point_lights`; risk: adding a fake green glow around every ore tree. |
| `LT_*` terrain object art is static terrain scenery, not a lamp source. | `rulesmd.ini` `[TerrainTypes]` registry and `LT_*` sections; `artmd.ini` `Foundation=1x1`; `0x0071DEA0` | No source-collection delta needed | map terrain object rendering / `src/map/overlay.rs` / `src/app_instances/overlays.rs` | Render as terrain object sprites and tint from cell lighting only. | A placed `LT_GEN01` terrain object does not illuminate adjacent cells unless another source lights those cells. | `terrain_lt_gen_object_does_not_emit_light`; risk: double-lighting maps if terrain object names are treated as lamps. |
| Terrain objects consume existing cell lighting during draw. | `TerrainClass__Draw_It @ 0x0071C1B0`; `LIGHTING_DRAW_CONSUMERS_CELL_FIELDS_GHIDRA_REPORT.md` | Partial broader renderer parity; current tint path is approximate | `src/app_instances/overlays.rs`, `src/map/lighting.rs` cell-light bundle work | Keep terrain object sprite lighting as a consumer of the resolved cell lighting model. | A building lamp near a tree changes the tree sprite's cell-light scalar/tint, but the tree itself does not expand the lit area. | `terrain_object_sprite_consumes_cell_lighting_without_emitting`; risk: removing terrain tint entirely while fixing source ownership. |

## Negative Facts / Do Not Do

- Do not parse `LightVisibility`, `LightIntensity`, or `Light*Tint` from `TerrainObjectType` as emitted light fields. Evidence: `TerrainTypeClass__ReadINI_Full @ 0x0071DEA0` has no such string refs.
- Do not make stock `TIBTRE01..03` emit green light from their stray `Light*` lines. Evidence: those INI lines exist, but the strings are owned by `BuildingTypeClass_ReadINI_Water` only.
- Do not make `LT_GEN*`, `LT_SGN*`, or `LT_EUR*` terrain objects emit light because their names/art look like light posts. Evidence: stock sections have no `Light*` lines and the terrain lifecycle has no LightSource caller.
- Do not add a terrain equivalent of `BuildingClass+0x614`. Evidence: `LightSourceClass__Constructor` callers are building Unlimbo, building construction complete, and RadSite activation only.
- Do not remove terrain object lighting consumption. Evidence: `TerrainClass__Draw_It @ 0x0071C1B0` reads cell light profile/scalars and passes scalar light to `CC_Draw_Shape`.

## Remaining Uncertainty

None for the scoped standard-YR terrain-object LightSource ownership question.

## Stale Docs / Follow-up Docs

- `C:/Users/enok/Documents/ra2-rust-game-docs/TERRAIN_CLASS_GHIDRA_REPORT.md` section 5 currently says: "Light-related keys on TIBTRE (`LightVisibility`, `LightIntensity`, `LightRedTint`, `LightGreenTint`, `LightBlueTint`) are not read by TerrainTypeClass::ReadINI_Full - they are consumed by a separate light-source parser; confirming the exact reader is out of scope for this report but is an obvious next investigation."
- Replacement wording: "Light-related keys on TIBTRE (`LightVisibility`, `LightIntensity`, `LightRedTint`, `LightGreenTint`, `LightBlueTint`) are not read by `TerrainTypeClass::ReadINI_Full` and are not consumed by any terrain light-source parser in standard YR. The five key strings are owned by `BuildingTypeClass_ReadINI_Water`; terrain objects consume existing cell lighting when drawn but do not create `LightSourceClass` data."

## Sources

- Ghidra decompiled: `0x0071DA80`, `0x0071DEA0`, `0x00665DE0`, `0x0071D000`, `0x0071C930`, `0x0071C730`, `0x0071C1B0`, `0x0045FE50`, `0x00440580`, `0x00554760`.
- Ghidra call graph: `LightSourceClass__Constructor` callers; `TerrainClass__Unlimbo`, `TerrainClass__Limbo`, `TerrainClass__AI`, and `TerrainTypeClass__ReadINI_Full` callees.
- Ghidra string anchors: `LightVisibility @ 0x0081A92C`, `LightIntensity @ 0x0081A91C`, `LightRedTint @ 0x0081A90C`, `LightGreenTint @ 0x0081A8FC`, `LightBlueTint @ 0x0081A8EC`, `SpawnsTiberium @ 0x00844674`, `IsAnimated @ 0x0084465C`, `TerrainTypes @ 0x00839DCC`.
- Existing docs: `LIGHT_RULES_ART_PARSER_DEFAULTS_GHIDRA_REPORT.md`, `TERRAIN_CLASS_GHIDRA_REPORT.md`, `TERRAINTYPECLASS_2B2_2B3_FLAGS_GHIDRA_REPORT.md`, `LIGHTSOURCE_LIFECYCLE_POWER_DAMAGE_SAVELOAD_GHIDRA_REPORT.md`, `BUILDING_LIGHTSOURCE_POST_LOAD_REHYDRATE_GHIDRA_REPORT.md`, `LIGHTING_DRAW_CONSUMERS_CELL_FIELDS_GHIDRA_REPORT.md`.
- INI checked: `ini/rulesmd.ini`, `ini/rules.ini`, `ini/artmd.ini`, `ini/art.ini`.
- Rust scanned: `src/rules/terrain_object_type.rs`, `src/rules/object_type.rs`, `src/rules/ruleset.rs`, `src/map/lighting.rs`, `src/app_init.rs`, `src/app_instances/overlays.rs`.

Status: COMPLETE.
