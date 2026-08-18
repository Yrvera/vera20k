# Building Type ID / Image Fallback Parity — Ghidra Research Report

**Address(es):** `0x00672660`, `0x004653C0`, `0x005F7090`, `0x005F92D0`, `0x0045FE50`, `0x0045F230`, `0x0045F040`  
**Investigation Mode:** exhaustive-slice  
**Claimed Scope:** building type identity from `[BuildingTypes]`, rules-section `Image=` storage, art-section `Image=` fallback/redirect for building body SHP lookup, and current Rust comparison.  
**Non-Scope:** owner, position, health, foundation semantics, building anim slots, damaged-art selection, sidebar cameo selection, voxel turret/barrel lookup except where call paths identify body image resolution.  
**Confidence:** High for binary behavior and Rust comparison points in this slice.  
**Active in YR:** Yes. Evidence: `RulesClass__ReadBuildingTypes @ 0x00672660` is the standard rules loader path; `BuildingTypeClass_ReadINI_Water @ 0x0045FE50` is the BuildingType vtable `ReadINI` entry (`0x007E45D4` xref); `BuildingTypeClass__LoadVisualAssets @ 0x0045F230` is called by `0x0045FE50`.

## 1. Overview

Buildings in gamemd.exe are keyed by the type name read from the `[BuildingTypes]` list. That name constructs or finds a `BuildingTypeClass`, becomes the canonical rules section name at `AbstractTypeClass+0x24`, and is copied to the object image buffer at `ObjectTypeClass+0x1F8` as the default.

`Image=` is a two-stage lookup. Rules `[BuildingType].Image=` changes the art section name stored at `+0x1F8`; art `[that section].Image=` then optionally redirects only the SHP filename loaded for the building body. If both are absent, the building body image falls back to the original type id.

## 2. Class Layout / Key Offsets

| Owner | Offset | Meaning | Evidence | Active in YR |
|---|---:|---|---|---|
| `AbstractTypeClass` | `+0x24` | canonical type/rules section name, copied from constructor argument | `AbstractTypeClass__Constructor @ 0x00410800` copies `param_2` into `param_1+9` | Yes: inherited by `BuildingTypeClass` |
| `ObjectTypeClass` | `+0x1F8` | image/art section id, default copied from `+0x24`; overwritten by rules `Image=` if present | `ObjectTypeClass__Constructor @ 0x005F7090`, `ObjectTypeClass__ReadINI @ 0x005F92D0` | Yes |
| `ObjectTypeClass` / `BuildingTypeClass` | `+0xA4` | loaded SHP pointer for primary image | `BuildingTypeClass__LoadVisualAssets @ 0x0045F230`, `BuildingTypeClass__GetImage @ 0x0045F040` | Yes |
| `BuildingTypeClass` | `+0x1760` | lazy-load/image-present flag for `+0xA4` path | `0x0045F040` checks it before lazy load | Conditional: only matters if the image pointer was not already loaded |
| `BuildingTypeClass` | `+0x176A` | cached body image filename built from art `Image=` or fallback | `0x0045F230` copies formatted filename to this buffer before loading `+0xA4` | Yes |

## 3. Core Logic

### Type identity

`RulesClass__ReadBuildingTypes @ 0x00672660` counts and iterates the `[BuildingTypes]` section. For each numeric entry, it reads the listed value into a local buffer and calls `BuildingTypeClass__FindOrAllocate`.

`BuildingTypeClass__FindOrAllocate @ 0x004653C0` searches `g_BuildingTypeClass_Array` by `existing_type + 0x24` against the requested name. On miss, it allocates `0x1798` bytes and calls `BuildingTypeClass__constructor(name)`.

`AbstractTypeClass__Constructor @ 0x00410800` stores the constructor name at `+0x24`, and `ObjectTypeClass__Constructor @ 0x005F7090` copies `+0x24` into `+0x1F8`. Therefore a building's type identity is the `[BuildingTypes]` object/section id, and the default image/art section is the same id.

**Active in YR:** Yes. This is the standard rules load path for all stock YR building type definitions. It is not gated by TS legacy flags.

### Rules `Image=` stage

`ObjectTypeClass__ReadINI @ 0x005F92D0` uses `param_1 + 9` (`+0x24`) as the rules section and calls `CCINIClass__ReadString(section, "Image", default=current +0x1F8, dest=+0x1F8, max=0x19)`.

If rules `Image=` is absent, `+0x1F8` remains the constructor-copied type id. If present, `+0x1F8` becomes the rules value. This is inherited by `BuildingTypeClass_ReadINI_Water @ 0x0045FE50`, which first calls `TechnoTypeClass__ReadINI`, which first calls `ObjectTypeClass__ReadINI`.

**Active in YR:** Yes. Stock YR uses this on buildings such as `[GAPRIS] Image=GAPRIS`, `[YABRCK] Image=YABRCK`, and many commented-out examples remain inactive because comments are not parsed.

### Art `Image=` stage

Most building art keys are read from section `+0x1F8`. `BuildingTypeClass_ReadINI_Water @ 0x0045FE50` reads many visual keys from `param_1 + 0x1F8`, while rules/gameplay keys use `param_1 + 0x24`. That establishes the two-section pattern: rules section is the type id, art section is the resolved image/art id.

`BuildingTypeClass__LoadVisualAssets @ 0x0045F230` reads `Image` from the art section named by `+0x1F8`, with empty-string default. It then tests the local result:

```text
if art Image= was absent/empty: filename base = this + 0x1F8
else: filename base = local art Image value
```

The chosen filename base is formatted with the current theater extension context, copied to `+0x176A`, and used to load the body SHP into `+0xA4`. `BuildingTypeClass__GetImage @ 0x0045F040` returns `+0xA4`, lazily loading from `+0x176A` if needed.

**Active in YR:** Yes. It is called by the active BuildingType `ReadINI` path. Example INI evidence: `rulesmd.ini:13091 [YACNST]` has rules `;Image=GACNST` commented out, and `artmd.ini:1622 [YACNST]` has art `;Image=GACNST` commented out, so YACNST falls back to `YACNST`. `artmd.ini:3228 [YAPOWR] Image=YAPOWR` is an explicit art value equal to its section, so it behaves the same as fallback.

## 4. INI Keys

| Key path | Binary consumer | Default/fallback | Active in YR |
|---|---|---|---|
| `[BuildingTypes] N=<TYPE>` | `RulesClass__ReadBuildingTypes @ 0x00672660` | no object if not listed | Yes |
| `[<TYPE>] Image=` in rules(md).ini | `ObjectTypeClass__ReadINI @ 0x005F92D0` | existing `+0x1F8`, initialized from `+0x24` | Yes |
| `[<resolved +0x1F8>] Image=` in art(md).ini | `BuildingTypeClass__LoadVisualAssets @ 0x0045F230` | `+0x1F8` when absent/empty | Yes |
| Other building art keys, e.g. `BibShape=`, anim keys | `BuildingTypeClass_ReadINI_Water @ 0x0045FE50` / `LoadVisualAssets @ 0x0045F230` | read from art section `+0x1F8`, not the final art `Image=` target | Yes, but detailed effects are out of scope |

## 5. Integration Points

`RulesClass__ReadBuildingTypes @ 0x00672660` registers the type object before per-type INI parsing. `BuildingTypeClass_ReadINI_Water @ 0x0045FE50` is reached through the BuildingType vtable and performs the two-section parse: rules keys from `+0x24`, art keys from `+0x1F8`. `BuildingTypeClass__LoadVisualAssets @ 0x0045F230` resolves the body SHP filename and loads/caches it. `BuildingTypeClass__GetImage @ 0x0045F040` returns the cached image pointer.

No TS-only gate was found in this slice. The lazy-load branch in `0x0045F040` is conditional on `+0x1760` and missing `+0xA4`, but that is a cache state, not a TS/YR feature gate.

## 6. Current Rust Implementation Status

| Rust point | Status | Evidence | Notes |
|---|---|---|---|
| `[BuildingTypes]` identity becomes object id | MATCH | `src/rules/ruleset.rs:45`, `src/rules/ruleset.rs:1332` | RuleSet maps `BuildingTypes` ids into `ObjectType` keyed by id. |
| Rules `Image=` defaults to type id | MATCH | `src/rules/object_type.rs:220`, `src/rules/object_type.rs:847` | `ObjectType.image = section.get("Image").unwrap_or(id)`. |
| Runtime entity stores rules type id, not image id | MATCH | `src/map/entities.rs:40`, `src/sim/world/world_spawn.rs:322`, `src/sim/game_entity.rs:291` | Map/spawn paths keep `type_id`/`type_ref`; render resolves image later. |
| Art `Image=` redirects effective SHP id | MATCH | `src/rules/art_data.rs:509`, `src/rules/art_data.rs:516`, `src/rules/art_data.rs:548`, `src/render/sprite_atlas.rs:1058` | `resolve_effective_image_id(type_id, rules_image)` applies art entry `Image=`. |
| Art metadata remains on base art section when present | MATCH | `src/rules/art_data.rs:521`, `src/rules/art_data_tests.rs:118`, `src/render/sprite_atlas.rs:1005` | This matches gamemd reading art metadata from `+0x1F8` while body filename can redirect via art `Image=`. |
| `RuleSet::merge_art_data` metadata lookup | UNKNOWN / narrow risk | `src/rules/ruleset.rs:1722` | It uses `art.get(obj.image).or_else(|| art.get(obj.id))` and does not apply art `Image=` recursively. This matches metadata-section lookup for foundations, but this report did not verify every merge-art consumer against binary. |

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `[BuildingTypes]` list reader | verified | `0x00672660`, string `0x00839DDC` | none |
| BuildingType find/allocate identity | verified | `0x004653C0`, compares `existing +0x24` | none |
| Type-name constructor storage | verified | `0x00410800`, `0x005F7090` | none |
| Rules `Image=` parser | verified | `0x005F92D0`, string `0x00819420` | none |
| BuildingType two-section parse | verified | `0x0045FE50`, rules keys from `+0x24`, art keys from `+0x1F8` | none |
| Art `Image=` body SHP redirect/fallback | verified | `0x0045F230`, string `0x00819420`, fallback to `+0x1F8` | none |
| Cached image getter | verified | `0x0045F040`, returns/loads `+0xA4` from `+0x176A` | none |
| Full damaged-art/body selection | deferred | `0x004513D0` touched | out-of-scope; requires separate damaged-art slice |
| Rust object identity and image fallback | verified | files/lines in Section 6 | none for this slice |

## 8. Open Questions — Final State of the Investigation Log

- `[RESOLVED] OQ-1 — Is building type identity the rules [BuildingTypes] listed object, not the art image? → Yes; [BuildingTypes] values feed FindOrAllocate and are stored at +0x24.` (evidence: `0x00672660`, `0x004653C0`, `0x00410800`)
- `[RESOLVED] OQ-2 — What initializes the image/art section when rules Image= is absent? → ObjectType ctor copies +0x24 into +0x1F8, and ObjectType ReadINI uses +0x1F8 as the default for Image.` (evidence: `0x005F7090`, `0x005F92D0`)
- `[RESOLVED] OQ-3 — Does rules Image= change type identity? → No; it writes +0x1F8 only. The +0x24 type name remains the rules section/type identity.` (evidence: `0x005F92D0`)
- `[RESOLVED] OQ-4 — Where are building art keys read? → BuildingType reads visual/art keys from section +0x1F8 after the rules stage resolves it.` (evidence: `0x0045FE50`)
- `[RESOLVED] OQ-5 — What does art Image= do for body SHP loading? → It overrides the filename base used by LoadVisualAssets; if absent/empty, the filename base falls back to +0x1F8.` (evidence: `0x0045F230`)
- `[RESOLVED] OQ-6 — Is the final SHP cached and returned through a building type getter? → Yes; +0x176A stores the formatted filename and +0xA4 stores the loaded SHP pointer returned by GetImage.` (evidence: `0x0045F230`, `0x0045F040`)
- `[RESOLVED] OQ-7 — Does Rust keep entity type_id distinct from image id? → Yes; spawn and map entities store the rules type id and rendering resolves image later.` (evidence: `src/map/entities.rs:40`, `src/sim/world/world_spawn.rs:322`, `src/render/sprite_atlas.rs:1058`)
- `[DEFERRED] OQ-8 — Does every non-body consumer in Rust match gamemd's metadata-vs-final-image distinction?` (category: out-of-scope; reason: this slot is limited to type id/body image fallback; next-step-if-pursued: audit cameo, palette, foundation, lighting, and production queue consumers as separate slices)

## Sources

- Ghidra: `0x00672660`, `0x004653C0`, `0x00410800`, `0x005F7090`, `0x005F92D0`, `0x00712170`, `0x0045FE50`, `0x0045F230`, `0x0045F040`, `0x004513D0` touched for non-scope boundary.
- INI: `ini/rulesmd.ini:13091`, `ini/rulesmd.ini:13125`, `ini/artmd.ini:1622`, `ini/artmd.ini:3228`.
- Rust: `src/rules/ruleset.rs`, `src/rules/object_type.rs`, `src/rules/art_data.rs`, `src/render/sprite_atlas.rs`, `src/map/entities.rs`, `src/sim/world/world_spawn.rs`, `src/sim/game_entity.rs`.
