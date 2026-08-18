# BUILDINGTYPE_EXTRALIGHT_0X1548_CONSUMER_GHIDRA_REPORT

Date: 2026-05-22  
Investigation mode: exhaustive-slice  
Target: `BuildingTypeClass+0x1548`, read from `ExtraLight=` in building art data

## Target Question

Identify the binary reader and consumer(s) of `BuildingTypeClass+0x1548`, associated with the `ExtraLight=` art key. Determine whether it is actually a lighting/ambience value or a rendering-depth value, including scale, sign, affected render paths, default behavior, and whether it is active in standard Yuri's Revenge.

## Non-goals

- Do not investigate `LightSourceClass`, `LightVisibility=`, `LightIntensity=`, or map `[Lighting]` ambience.
- Do not investigate active anim light flags except where needed to prove `ExtraLight=` is separate.
- Do not modify Rust, INI data, or existing research docs.
- Do not rename or mutate Ghidra symbols.

## Evidence Needed To Mark COMPLETE

- Reader function and exact key string xref for `ExtraLight=`.
- Storage width, signedness, default, and parse behavior for `BuildingTypeClass+0x1548`.
- Binary consumer(s) of `BuildingTypeClass+0x1548` in player-visible render paths.
- Negative proof that this key is not consumed by map lighting/light-grid code in the observed binary slice.
- Standard YR activity status for each material finding.

## Stop Conditions

- Stop when all direct code references to the `ExtraLight` string are accounted for.
- Stop when all found code references to the literal `+0x1548` that could plausibly be `BuildingTypeClass+0x1548` are classified.
- Stop without runtime watchpoints if static xrefs are sufficient to identify reader and consumers.

## Prior Work And Conflicts

- `BUILDINGTYPECLASS_CTOR_DEFAULTS.md` and `BUILDINGTYPECLASS_FIELDS.csv` identify `BuildingTypeClass+0x1548` as `ExtraLight`, default `0`, written as a word in the constructor.
- `BUILDINGCLASS_DRAWBODY_GHIDRA_REPORT.md` identifies the same offset as `ExtraZAdjust` / draw-depth adjustment in `BuildingClass_DrawBody`.
- Current Rust parses `ExtraLight=` in `src/rules/art_data.rs` and applies it as flat RGB cell brightness in `src/map/lighting.rs::apply_extra_light`.

The binary resolves the naming conflict: the INI key name is `ExtraLight`, but the stored value is consumed as signed SHP draw depth/Z adjustment, not as a map-light brightness contribution.

## Verified Binary Findings

### 1. `ExtraLight` string has one code xref, in `BuildingTypeClass_ReadINI_Water`

Evidence:

- Binary string scan of retail `gamemd.exe` found `ExtraLight\0` at VA `0x0081A650`.
- The only little-endian immediate reference to `0x0081A650` in `.text` is at `0x004613F2`, inside `BuildingTypeClass_ReadINI_Water @ 0x0045FE50`.
- Nearby strings are `ZShapePointMove` at `0x0081A65C` and `CanHideThings` at `0x0081A640`, matching the building art/read table neighborhood.

Reader sequence around `0x004613EC..0x00461404`:

```asm
0F BF 95 48 15 00 00   movsx edx, word ptr [ebp+1548h]
52                     push edx                    ; default value
68 50 A6 81 00         push 0081A650h              ; "ExtraLight"
57                     push edi                    ; section id/name
B9 80 71 88 00         mov ecx, 00887180h          ; CCINI instance
E8 CF 62 0C 00         call CCINIClass__ReadInt
66 89 85 48 15 00 00   mov word ptr [ebp+1548h], ax
```

Claim: `ExtraLight=` is read as an integer using the existing signed 16-bit field value as default, then truncated/stored as a 16-bit word at `BuildingTypeClass+0x1548`.  
Active in YR: Yes. This is in the standard building type INI reader used for retail YR building types.

### 2. Constructor default is zero and the field is 16-bit

Evidence:

- `BuildingTypeClass__constructor @ 0x0045DF00` decompile writes `*(undefined2 *)(param_1 + 0x552) = 0`, i.e. byte offset `0x1548`.
- Prior default table cites the same constructor write at `0x0045DFCC`.

Claim: Absent `ExtraLight=`, `BuildingTypeClass+0x1548` defaults to `0`.  
Active in YR: Yes. All building type instances pass through this constructor path.

### 3. Normal building body rendering consumes `+0x1548` as signed depth/Z adjustment

Evidence:

- `BuildingClass_DrawBody @ 0x0043D290` reads the field via signed-short casts:
  - main body draw: `cell+0x10A + (short)(Type+0x1548)`
  - rubble/alternate body draw: same signed-short addition
  - damaged/auxiliary body draw branches: same signed-short addition
- The result is passed into `vtable+0x1D0(...)`, `Tactical__AdjustForZ`, and `TechnoClass_DrawSHP(...)`.
- No RGB, `LightConvertClass`, `LightSourceClass`, or cell lighting data is touched at these read sites.

Claim: In the live building draw path, `ExtraLight=` changes sprite draw depth/Z adjustment, not cell brightness. Positive values increase the signed depth value passed to the draw pipeline; negative values decrease it. There is no `/1000` scale in this path.  
Active in YR: Yes. `BuildingClass_DrawBody` is the standard visible building body renderer.

### 4. A fogged-object/cached-building render path also consumes `+0x1548` as signed depth

Evidence:

- `FUN_004D1890` contains a building-like cached draw path. It loads a building type pointer and computes:

```c
iVar16 = (int)*(short *)(cell + 0x10a) + (int)(short)piVar9[0x552];
```

- `piVar9[0x552]` is the same byte offset `0x1548`.
- The result is passed to `CC_Draw_Shape(...)` as the depth/Z argument.

Claim: The cached/fogged building rendering path also treats `+0x1548` as signed depth adjustment.  
Active in YR: Conditional. The code is present in `gamemd.exe`, but it belongs to a fogged/cached object rendering path. Standard YR has TS-style fog of war disabled by default, so do not treat this as the ordinary visible-building path unless a map/rules setup enables the relevant fogged-object behavior.

### 5. No direct lighting consumer was found for `ExtraLight=`

Evidence:

- The `ExtraLight` string has exactly one code xref, the reader at `0x004613F2`.
- The relevant `BuildingTypeClass+0x1548` references found in `.text` resolve to:
  - constructor default write
  - `BuildingTypeClass_ReadINI_Water` default/read/write
  - `BuildingClass_DrawBody` signed-depth consumers
  - `FUN_004D1890` cached/fogged signed-depth consumer
  - unrelated `RulesClass+0x1548` difficulty fields in `HouseClass__SetDifficulty`
  - unrelated UI/string-table numeric ID use around `0x005EB060`
- No observed reference routes `+0x1548` into `LightSourceClass`, `LightConvertClass`, map cell RGB ambience, palette brightness, or a lighting grid.

Claim: For this bounded slice, `ExtraLight=` is not a map-light ambience key. It should not be used to brighten/darken map cells.  
Active in YR: Yes as a negative fact for standard visible rendering; no standard YR lighting consumer was found.

## INI Data

Retail RA2 and YR art data contain only four observed `ExtraLight=` building entries in the checked files:

| Section | Value | Nearby art context | Binary effect |
|---|---:|---|---|
| `GADPSA` | `-100` | 1x1 deployable sensor array, active anim z adjust `-100` | signed draw-depth decrease |
| `GAICBM` | `-100` | 1x1 deployable ICBM launcher, active anim z adjust `-100` | signed draw-depth decrease |
| `GATICK` | `-100` | Tick Tank deployed building art | signed draw-depth decrease |
| `GAARTY` | `350` | deployed artillery building art | signed draw-depth increase |

Active in YR: Conditional by unit/art availability. The keys are in `artmd.ini`; the parser path is active. Whether each section appears in normal play depends on deployed-unit/building usage and game mode/mod content.

## Player-visible Effect

`ExtraLight=` affects render ordering/depth placement of the building SHP and related body layers. The visible symptom of a wrong implementation is not a brighter or darker cell; it is sprites sorting in front of/behind the wrong terrain, bridge deck, nearby objects, brackets, or overlays.

The retail values make sense as art-order corrections:

- `-100` raises/lowers the draw-depth relationship for small deployed structures that otherwise sort incorrectly.
- `350` pushes the deployed artillery art in the opposite direction.

This is an interpretation from the verified draw-depth consumer, not a separate runtime screenshot comparison. The binary fact is the signed-depth addition.

## Negative Facts / Do Not Do

- Do not apply `ExtraLight=` to `LightingGrid` or per-cell RGB tint.
- Do not divide `ExtraLight` by `1000`; the binary uses the raw signed integer stored as a signed 16-bit value.
- Do not treat `ExtraLight` as a radius light, palette light, `LightSourceClass`, or `LightConvertClass` input.
- Do not spread it across foundation cells.
- Do not apply it to map `[Lighting]` ambience.
- Do not assume the field name `ExtraLight` describes the runtime effect. The observed consumer semantics are Z/depth adjustment.

## Current Rust Mismatch

Rust currently:

- parses `ExtraLight=` in `src/rules/art_data.rs` as `ArtEntry.extra_light`;
- documents it as extra ambient light;
- applies it in `src/map/lighting.rs::apply_extra_light` as a flat brightness adjustment on the building's own cell with scale `1000 ~= 1.0`;
- calls that from `src/app_init.rs` after point lights.

This conflicts with the binary. The key should be carried into building render-depth data, not into map lighting.

## Implementation Handoff

Affected Rust surfaces:

- `src/rules/art_data.rs`: keep parsing the key, but rename or reinterpret the semantic away from ambient light. Since the INI key name is `ExtraLight`, consider an internal name like `building_extra_z_adjust` or `extra_light_z_adjust` with a comment explaining the binary naming trap.
- `src/map/lighting.rs`: remove `apply_extra_light` from lighting behavior or stop feeding `ExtraLight=` into RGB tint.
- `src/app_init.rs`: remove the `lighting::apply_extra_light` call once renderer-side depth support exists.
- building sprite instance/depth generation: apply signed `ExtraLight` to the same depth path that uses cell `+0x10A`/height-derived Z adjustment for building body draw ordering.
- selection bracket/building overlay depth: keep depth in sync with the body if Rust has split calculations, matching the existing DrawBody research implication.

Concrete test-name proposals:

- `art_extra_light_parses_signed_i16_depth_adjust`
- `building_extra_light_does_not_modify_lighting_grid`
- `building_depth_includes_extra_light_z_adjust`
- `deployed_artillery_extra_light_positive_changes_sort_depth`
- `deployed_sensor_extra_light_negative_changes_sort_depth`
- `lighting_grid_ignores_art_extra_light_even_when_structure_present`

Acceptance scenarios:

- A map/entity using `GAARTY` gets a positive building render-depth adjustment of `350`, with no RGB/tint change to its cell.
- A map/entity using `GADPSA`, `GAICBM`, or `GATICK` gets a negative building render-depth adjustment of `-100`, with no RGB/tint change to its cell.
- Removing `ExtraLight=` from an art section falls back to `0`.
- Values outside signed 16-bit range should be handled deliberately; binary stores low 16 bits and later sign-extends. For mod parity, document whether Rust clamps, wraps, or rejects.

## Remaining Uncertainty

- Runtime screenshot comparison was not performed in this slot. The consumer and semantics are static-binary verified, but exact visual examples should be validated later with an art/map fixture.
- The fogged-object path in `FUN_004D1890` was classified only enough to identify its `+0x1548` use. A separate fog-of-war investigation should decide whether/how Rust needs that path.
- The exact relationship between this value and all Rust renderer depth units needs integration design. The binary uses the raw signed value in the same depth coordinate passed to draw calls; Rust may have a normalized depth representation.

## Status

COMPLETE for the bounded question. `ExtraLight=` is a binary-verified reader for `BuildingTypeClass+0x1548`, but the field's active standard YR consumer is building SHP draw depth/Z adjustment, not map lighting ambience.
