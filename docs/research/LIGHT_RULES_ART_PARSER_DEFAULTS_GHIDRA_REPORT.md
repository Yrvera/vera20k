# Light Rules / Art Parser Defaults - Ghidra Research Report

**Address(es):** `0x0045DD90`, `0x0045FE50`, `0x005283D0`, `0x006832C0`, `0x00683610`, `0x00689E90`, `0x0043D290`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** binary/INI/Rust parser defaults and ownership for `LightVisibility`, `LightIntensity`, `LightRedTint`, `LightGreenTint`, `LightBlueTint`, `ExtraLight`, and map `[Lighting]` defaults.  
**Non-Scope:** LightSource falloff math, dirty-cell scheduling, LightConvert palette generation, spotlight behavior beyond key separation, Lightning Storm, radiation glow, and superweapon lighting transitions except where `[Lighting]` parser fields prove ownership.  
**Confidence:** High for defaults, key ownership, `ExtraLight` negative lighting fact, and `CCINIClass__ReadDouble` malformed-value behavior; Medium for exact BuildingType light-key reader scale because `0x0045FE50` is very large and this pass used string xrefs plus ctor/field docs rather than a full reader-line drain.  
**Active in YR:** Yes for standard building/map/art readers; Conditional for player-visible lamp output because a placed building needs nonzero `LightIntensity`.

## Target Question

Which binary class parser owns the map-lighting-related INI keys, what defaults and internal scales do they use, and how should Rust treat stock malformed values such as `LightGreenTint=0,01`?

## Non-Goals

- Do not implement or edit Rust.
- Do not patch existing docs or `.swarm-claims.md`.
- Do not investigate final cell light propagation, LightConvert cache internals, or dynamic update timing.
- Do not treat `ExtraLight=` as a lighting consumer question beyond proving its parser/default and checked binary consumer semantics.

## Evidence Needed To Mark COMPLETE

- Decompile constructor/default writers for `BuildingTypeClass` and `ScenarioClass`.
- Locate binary string anchors for all target keys and identify the owning parser where possible.
- Decompile `CCINIClass__ReadDouble` to determine malformed float behavior.
- Check stock INI examples for lamp values and `ExtraLight=`.
- Scan current Rust parser surfaces and name deltas/test proposals.
- Confirm `ExtraLight=` is or is not consumed as RGB ambience.

## Stop Conditions

- Stop after parser/default evidence and one direct `ExtraLight` consumer check.
- Defer exact LightSource per-cell math and LightConvert normalization to the dedicated lighting reports.
- Stop before mutating Ghidra state; all Ghidra work in this pass was read-only.

## 1. Overview

Lamp-post ambience keys live on `BuildingTypeClass`, not map files and not `TerrainTypeClass`. `LightVisibility` defaults to `5000`; `LightIntensity` defaults to zero; the RGB tints default to 1.0-equivalent fixed-point values (`1000000`). Those defaults mean ordinary buildings do not become lamps merely because visibility defaults nonzero.

Map `[Lighting]` is parsed by `ScenarioClass__Read_INI_Basic`, with constructor defaults stored in integer fields before the map section overrides them. `ExtraLight=` is parsed from art/image metadata but is not map RGB ambience: the checked binary consumer in `BuildingClass_DrawBody` uses `Type+0x1548` as a signed draw-depth/Z adjustment.

## 2. Class Layout / Key Offsets

| Field / key | Owner | Offset | Default | Evidence | Active in YR |
|---|---:|---:|---:|---|---|
| `LightVisibility=` | `BuildingTypeClass` rules section | `+0xE30` | `5000` | ctor `0x0045DD90` writes `param_1[0x38c]=5000`; `BUILDINGTYPECLASS_CTOR_DEFAULTS.md` | Yes, but visible only when paired with nonzero intensity |
| `LightIntensity=` | `BuildingTypeClass` rules section | `+0xE34` | `0` | ctor `0x0045DD90` writes `param_1[0x38d]=0`; Unlimbo gate reads `Type+0xE34` | Yes, gates lamp allocation |
| `LightRedTint=` | `BuildingTypeClass` rules section | `+0xE38` | `1000000` | ctor `0x0045DD90` writes `param_1[0x38e]=1000000`; string `0x0081A90C` | Yes |
| `LightGreenTint=` | `BuildingTypeClass` rules section | `+0xE3C` | `1000000` | ctor `0x0045DD90` writes `param_1[0x38f]=1000000`; string `0x0081A8FC` | Yes |
| `LightBlueTint=` | `BuildingTypeClass` rules section | `+0xE40` | `1000000` | ctor `0x0045DD90` writes `param_1[0x390]=1000000`; string `0x0081A8EC` | Yes |
| `ExtraLight=` | Building image/art metadata merged into `BuildingTypeClass` | `+0x1548` signed short | `0` | ctor `0x0045DD90` writes word zero; string `0x0081A650`; DrawBody reads `Type+0x1548` | Yes, as draw-depth/Z adjustment |
| `[Lighting] Ambient` | `ScenarioClass` map reader | `+0x3528`; copies to `+0x352C/+0x3530` | internal `100` | `0x00683610`, `0x00689E90` | Yes |
| `[Lighting] Red` | `ScenarioClass` map reader | `+0x3534` | internal `100` | `0x00683610`, `0x00689E90` | Yes |
| `[Lighting] Green` | `ScenarioClass` map reader | `+0x3538` | internal `100` | `0x00683610`, `0x00689E90` | Yes |
| `[Lighting] Blue` | `ScenarioClass` map reader | `+0x353C` | internal `100` | `0x00683610`, `0x00689E90` | Yes |
| `[Lighting] Ground` | `ScenarioClass` map reader | `+0x3540` | internal `50` | `0x00683610`, `0x00689E90` | Yes |
| `[Lighting] Level` | `ScenarioClass` map reader | `+0x3544` | internal `8` | `0x00683610`, `0x00689E90` | Yes |

## 3. Core Logic

### Building light-key defaults

`BuildingTypeClass__constructor @ 0x0045DD90` initializes the light-related block before INI reads:

- `+0xE30 = 5000`.
- `+0xE34 = 0`.
- `+0xE38/+0xE3C/+0xE40 = 1000000`.

The runtime allocation check settled by the parent synthesis and prior Unlimbo spot-check is `Type+0xE34 != 0`. Therefore `LightVisibility=5000` is a real data default, but it does not make every standard building emit a light. Active in YR: Yes, conditional on nonzero intensity.

### Malformed float handling

`CCINIClass__ReadDouble @ 0x005283D0`:

1. Finds the section and key through the INI hash/cache path.
2. If missing, returns the caller-provided default.
3. If present, runs `sscanf(raw_value, "%f", &temp_float)`.
4. If the raw string contains `%`, multiplies the parsed float by `0.01`.
5. It does not require full-string consumption.

This means a value like `0,01` is not rejected as missing and is not parsed as `0.01` in the normal C locale. `%f` consumes the leading `0`, stops at the comma, and returns `0.0`. Active in YR: Yes, for all `ReadDouble`-based scalar reads, including lighting keys.

### Map `[Lighting]` defaults and reads

`ScenarioClass__Constructor @ 0x006832C0` calls `FUN_00683610`, which initializes lighting fields:

- normal map lighting: Ambient/R/G/B internal `100`, Ground `50`, Level `8`;
- Ion and Dominator lighting fields also receive defaults, but those variants are outside this slot's map ambience scope.

`ScenarioClass__Read_INI_Basic @ 0x00689E90` reads `[Lighting]` keys by passing current field-derived defaults into `CCINIClass__ReadDouble`, then stores integer fields back into the Scenario instance. Active in YR: Yes for every standard map load.

### `ExtraLight=` is not ambience

The art/image field named `ExtraLight=` is stored at `BuildingTypeClass+0x1548`, default `0`, signed short. In `BuildingClass_DrawBody @ 0x0043D290`, normal body and damaged-bib draw paths compute:

- `cell_depth = MapClass::Get_CellClass(...)->+0x10A`;
- `draw_depth = cell_depth + (short)Type+0x1548`;
- pass `draw_depth` to the draw-depth/vtable path and `TechnoClass_DrawSHP`.

No RGB lighting grid, `LightSourceClass`, or `LightConvertClass` use was found in this checked consumer. Active in YR: Yes, for buildings with art `ExtraLight=` such as `GADPSA=-100`, `GAICBM=-100`, `GATICK=-100`, and `GAARTY=350`.

## 4. INI Keys

| Key | INI owner | Stock examples | Binary default / parse | Rust delta | Active in YR |
|---|---|---|---|---|---|
| `LightVisibility` | building rules sections | `GALITE=5000`, colored lamps `3000..5000` | default `5000`, integer leptons | Rust defaults to `0` | Yes |
| `LightIntensity` | building rules sections | `0.2`, `0.01`, negative `-0.15` | default `0`; fixed/scaled numeric field | Rust stores raw `f32` | Yes |
| `LightRedTint` | building rules sections | `0.01`, `1.5`, `2.0`, negative on `NEGRED` | default 1.0-equivalent `1000000` | Rust raw `f32` default `1.0` | Yes |
| `LightGreenTint` | building rules sections | `0.05`, `1.5`, malformed `0,01` | malformed `0,01` parses as `0.0`, not default | Rust parse fails and falls back to `1.0` | Yes |
| `LightBlueTint` | building rules sections | `0.01`, `0.7`, `2.0`, negative snow lamps | default 1.0-equivalent `1000000` | Rust raw `f32` default `1.0` | Yes |
| `ExtraLight` | art/image sections | `GADPSA=-100`, `GAICBM=-100`, `GATICK=-100`, `GAARTY=350` | default `0`, signed short, draw-depth use | Rust treats as RGB brightness `/1000` | Yes |
| `[Lighting] Ambient/Red/Green/Blue/Ground/Level` | map scenario INI | map-provided when present | defaults from `FUN_00683610`; read by `0x00689E90` | Rust defaults differ for at least `Ground` | Yes |

Negative INI fact: `LightVisibility/LightIntensity/*Tint` lines on terrain sections such as TIBTRE are not proven to be owned by `TerrainTypeClass`; prior terrain docs say `TerrainTypeClass::ReadINI_Full` does not consume them. Active in YR for terrain: Deferred/out-of-scope.

## 5. Integration Points

- `BuildingTypeClass_ReadINI_Water @ 0x0045FE50` calls `TechnoTypeClass__ReadINI` first, then reads building-specific keys. String anchors for all five lamp fields have one binary reference each and are in the building-type parser region according to existing field reports.
- `BuildingClass__Unlimbo @ 0x00440580` allocates `LightSourceClass` at building runtime only when `Type+0xE34 != 0`, then copies `Type+0xE30/+0xE34/+0xE38/+0xE3C/+0xE40` into the constructor.
- `ScenarioClass__Read_INI_Basic @ 0x00689E90` owns map `[Lighting]`.
- `BuildingClass_DrawBody @ 0x0043D290` is a verified consumer of `ExtraLight=` as draw depth.
- `CCINIClass__ReadDouble @ 0x005283D0` is the scalar parser used by map lighting and many rules/art numeric fields.

## 6. Current Rust Implementation Status

- `src/rules/object_type.rs` parses light keys into `ObjectType::light_visibility/light_intensity/light_*_tint`; default `LightVisibility` is `0`, not binary `5000`.
- `src/rules/ini_parser.rs::get_f32` uses Rust `parse::<f32>()`; `0,01` returns `None` and callers use their default. Binary `ReadDouble` returns parsed leading `0.0`.
- `src/rules/art_data.rs` parses `ExtraLight` as `i32`, which preserves values, but comments call it ambient light.
- `src/map/lighting.rs::apply_extra_light` applies `ExtraLight / 1000.0` into RGB cell lighting. This is contradicted by `BuildingClass_DrawBody`.
- `src/map/lighting.rs::parse_lighting` parses map `[Lighting]` as raw `f32` defaults; binary defaults are integer ScenarioClass fields with `Ground=50`, `Level=8` internally.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `BuildingTypeClass` light defaults | verified | `0x0045DD90`; `BUILDINGTYPECLASS_CTOR_DEFAULTS.md` | none |
| Five light key string anchors | verified | strings `0x0081A92C`, `0x0081A91C`, `0x0081A90C`, `0x0081A8FC`, `0x0081A8EC`; one reference each | exact per-key reader line in huge `0x0045FE50` not separately sliced |
| `LightIntensity` allocation gate | verified by parent context | `BuildingClass__Unlimbo @ 0x00440580` | none for parser/default slice |
| `CCINIClass__ReadDouble` malformed float behavior | verified | `0x005283D0` | locale/runtime config not separately observed; standard C `%f` semantics apply |
| map `[Lighting]` defaults | verified | `0x00683610` | exact public-unit conversion for Ground/Level constants belongs to lighting formula slice |
| map `[Lighting]` reader | verified | `0x00689E90` | none for ownership/default slice |
| `ExtraLight` default | verified | `0x0045DD90`; `BUILDINGTYPECLASS_CTOR_DEFAULTS.md` | none |
| `ExtraLight` lighting-vs-depth consumer | verified | `0x0043D290`; `BUILDINGCLASS_DRAWBODY_GHIDRA_REPORT.md` | final Rust insertion point belongs to render-depth implementation pass |
| Rust parser deltas | verified | `src/rules/object_type.rs`, `src/rules/ini_parser.rs`, `src/rules/art_data.rs`, `src/map/lighting.rs` | no code changes made |
| Terrain light-key ownership | deferred | `TERRAIN_CLASS_GHIDRA_REPORT.md` | separate terrain-light parser investigation if needed |

## 8. Open Questions - Final State of the Investigation Log

- `[RESOLVED] LRD-01 - Which parser owns building lamp keys? -> Building-type rules parsing owns them; string anchors have single references and offsets are in `BuildingTypeClass+0xE30..0xE40`.` (evidence: `0x0045FE50`, string search, `BUILDINGTYPECLASS_FIELDS.csv`)`
- `[RESOLVED] LRD-02 - What is `LightVisibility` default? -> `5000`.` (evidence: `0x0045DD90`)`
- `[RESOLVED] LRD-03 - Does default visibility allocate lights? -> No by itself; runtime allocation gate is nonzero `LightIntensity`.` (evidence: `0x00440580`)`
- `[RESOLVED] LRD-04 - What are tint defaults? -> `1000000` fixed-point, 1.0-equivalent.` (evidence: `0x0045DD90`)`
- `[RESOLVED] LRD-05 - What happens to `0,01`? -> `CCINIClass__ReadDouble` uses `%f` and accepts the leading `0`, so binary value is `0.0`, not fallback and not `0.01`.` (evidence: `0x005283D0`)`
- `[RESOLVED] LRD-06 - Who owns `ExtraLight=`? -> Art/image metadata merged into `BuildingTypeClass+0x1548`, default signed short `0`.` (evidence: string `0x0081A650`, `0x0045DD90`, `BUILDINGTYPECLASS_FIELDS.csv`)`
- `[RESOLVED] LRD-07 - Is `ExtraLight=` RGB ambience? -> No checked consumer uses it as draw-depth/Z adjustment in `BuildingClass_DrawBody`.` (evidence: `0x0043D290`)`
- `[RESOLVED] LRD-08 - Who owns map `[Lighting]`? -> `ScenarioClass__Read_INI_Basic`.` (evidence: `0x00689E90`)`
- `[RESOLVED] LRD-09 - What are map lighting constructor defaults? -> Ambient/R/G/B `100`, Ground `50`, Level `8` internally.` (evidence: `0x00683610`)`
- `[RESOLVED] LRD-10 - Are Ion/Dominator lighting fields in this same parser? -> Yes, but they are outside normal map ambience scope.` (evidence: `0x00689E90`)`
- `[RESOLVED] LRD-11 - Does Rust match malformed `0,01`? -> No; Rust `parse::<f32>()` fails and falls back to the caller default.` (evidence: `src/rules/ini_parser.rs`)`
- `[RESOLVED] LRD-12 - Does Rust match `ExtraLight` semantics? -> No; Rust applies it as RGB brightness.` (evidence: `src/map/lighting.rs`; `0x0043D290`)`
- `[DEFERRED] LRD-13 - Exact public conversion constants for Scenario `Ground`/`Level`.` (category: requires-different-system-context; reason: belongs with map light formula/LightConvert slice; next-step-if-pursued: decompile the cell light compute path and constants around `_DAT_007f0e78/_DAT_007e3818`)`
- `[DEFERRED] LRD-14 - Terrain section light keys such as TIBTRE.` (category: out-of-scope; reason: target was map/building light parser defaults; next-step-if-pursued: run a terrain light-source parser slice)`
- `[DEFERRED] LRD-15 - Exact reader-line scale for `LightIntensity/*Tint` in the huge building reader.` (category: bounded-cost-too-high; reason: defaults/offsets/use are enough for this parser-default handoff, but the 180KB reader needs a narrow xref assembly pass for exact multiply constants; next-step-if-pursued: slice `0x0045FE50` around string refs `0x0081A91C..0x0081A8EC`)`

## 9. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| `LightVisibility` defaults to `5000`, while intensity defaults to zero | `0x0045DD90` | mismatch | `src/rules/object_type.rs` | Parse absent `LightVisibility=` as `5000` | `object_type_light_visibility_defaults_to_5000` | Do not use visibility alone as lamp gate |
| Lamp collection/allocation is intensity-gated | `0x00440580` | logic mostly matches; comments stale | `src/map/lighting.rs::collect_building_lights` | Keep `LightIntensity == 0` as no-light gate; update comments/tests | `building_light_collection_gates_on_nonzero_intensity_even_with_default_visibility` | Do not allocate lights for ordinary default buildings |
| `0,01` parses as `0.0` through binary `ReadDouble`, not `0.01` and not default | `0x005283D0`; `rulesmd.ini` lamp values | mismatch | `src/rules/ini_parser.rs::get_f32` and callers | If mimicking `ReadDouble`, allow partial float parse with comma stopping behavior for numeric keys using that reader | `ini_get_f32_comma_decimal_matches_binary_partial_parse` | Do not "fix" stock typo to `0.01` unless intentionally diverging from gamemd |
| `ExtraLight=` is signed depth/Z adjustment, not RGB light | `0x0043D290`; `0x0045DD90` | mismatch | `src/rules/art_data.rs`, `src/map/lighting.rs`, `src/app_init.rs`, building draw-depth code | Preserve parsed signed value; remove RGB-grid application; apply to building body depth in render path | `map_lighting_does_not_apply_extra_light_to_rgb_grid`; `building_extra_light_affects_depth_only` | Do not divide by `1000`; do not feed `LightSourceClass` or LightConvert |
| Map `[Lighting]` defaults come from `ScenarioClass` integer fields | `0x00683610`, `0x00689E90` | partial/unchecked | `src/map/lighting.rs::LightingConfig` | Align defaults and scaling only after the formula/constants slice confirms public-unit conversion | `map_lighting_defaults_match_scenario_constructor_units` | Do not assume Rust's `Ground=0.0` default is binary-correct |
| RGB tint defaults are 1.0-equivalent fixed point (`1000000`) | `0x0045DD90` | representation differs but default value intent matches | `src/rules/object_type.rs` | Keep default visible scalar `1.0`; document fixed-point source if storing floats | `object_type_light_tints_default_to_one_equivalent` | Do not treat ctor literal `1000000` as raw render multiplier |

### Negative Facts / Do Not Do

- Do not treat `ExtraLight=` as ambience, lamp radius, LightConvert input, or per-cell RGB boost.
- Do not parse `0,01` as `0.01` if the goal is gamemd parser parity.
- Do not move lamp keys to terrain parsing without a separate terrain proof.
- Do not assume map `[Lighting]` default `Ground=0.0`; binary constructor initializes the internal ground field to `50`.
- Do not use `LightVisibility > 0` alone as the LightSource creation condition.

### Stale Docs / Follow-up Docs

- Any doc saying `LightVisibility > 0` creates `LightSourceClass` should be replaced with: "LightSourceClass creation in checked Unlimbo is gated by nonzero `Type+0xE34` (`LightIntensity`); `LightVisibility` supplies radius and defaults to `5000`."
- Any doc/Rust comment saying `ExtraLight=` is ambient light should be replaced with: "`ExtraLight=` is a signed building art draw-depth/Z adjustment stored at `BuildingTypeClass+0x1548`; it must not modify map RGB ambience."
- The current handoff-audit test proposal `ini_get_f32_accepts_comma_decimal_light_values` should be corrected: binary parity expects `0,01` to produce `0.0` under `ReadDouble` partial `%f` parsing, not `0.01`.

## Sources

- Ghidra decompiled: `0x0045DD90`, `0x0045FE50`, `0x005283D0`, `0x006832C0`, `0x00683610`, `0x00689E90`, `0x0043D290`.
- Ghidra string search: `LightVisibility @ 0x0081A92C`, `LightIntensity @ 0x0081A91C`, `LightRedTint @ 0x0081A90C`, `LightGreenTint @ 0x0081A8FC`, `LightBlueTint @ 0x0081A8EC`, `ExtraLight @ 0x0081A650`.
- Existing docs: `BUILDINGTYPECLASS_CTOR_DEFAULTS.md`, `BUILDINGTYPECLASS_FIELDS.csv`, `BUILDINGCLASS_DRAWBODY_GHIDRA_REPORT.md`, `BUILDINGCLASS_UNLIMBO_AND_PLACEMENT.md`, `TERRAIN_CLASS_GHIDRA_REPORT.md`, `MAP_LIGHTCONVERT_CACHE_00483E30_00544E70_GHIDRA_REPORT.md`, `MAP_LIGHTING_RUST_HANDOFF_AUDIT.md`.
- INI checked: `ini/rulesmd.ini`, `ini/rules.ini`, `ini/artmd.ini`, `ini/art.ini`.
- Rust scanned: `src/rules/object_type.rs`, `src/rules/ini_parser.rs`, `src/rules/art_data.rs`, `src/map/lighting.rs`, `src/app_init.rs`.
