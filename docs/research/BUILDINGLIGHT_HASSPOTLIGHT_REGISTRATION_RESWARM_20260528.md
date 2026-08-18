# BuildingLight HasSpotlight Registration -- Reswarm Research Report

**Address(es):** `BuildingTypeClass_ReadINI_Water @ 0x0045FE50`, `BuildingClass__Unlimbo @ 0x00440580`, `BuildingLightClass__Constructor @ 0x00435820`, `FUN_00437050 @ 0x00437050`, `FUN_0055BAA0 @ 0x0055BAA0`
**Investigation Mode:** exhaustive-slice
**Claimed Scope:** `HasSpotlight=` reader/default, `BuildingTypeClass+0x154B` field identity, `BuildingClass+0x600` allocation gate, `BuildingLightClass` constructor/reveal active-list registration, standard YR activation status, and Rust handoff.
**Non-Scope:** WaveClass, OpenTopped passengers, full spotlight AI/draw pixel rasterization, cell action `0x23`, ordinary `LightSourceClass` ambience at `BuildingClass+0x614`, and Rust implementation.
**Confidence:** High
**Active in YR:** Conditional. The parser and runtime path are live in `gamemd.exe`, but standard repo `rules.ini`/`rulesmd.ini`/`art.ini`/`artmd.ini` and visible files under the configured retail install contain no `HasSpotlight=` assignments. Activation requires a map/mod/rules override that sets `HasSpotlight=yes` on a building type and then places/unlimbos that building.

## 1. Overview

`HasSpotlight=` is a building-type boolean parsed into `BuildingTypeClass+0x154B`. When that byte is nonzero, `BuildingClass__Unlimbo` allocates a `0xE8` `BuildingLightClass`, stores the pointer at `BuildingClass+0x600`, and the `BuildingLightClass` constructor explicitly registers the light object into the same `LogicClass` active vector through `FUN_0055BAA0` after a successful `ObjectClass__Reveal`.

This path is not the ordinary building point-light ambience system. `BuildingClass+0x614` / `LightSourceClass` is separately gated by `LightIntensity`/`LightVisibility` fields; `BuildingClass+0x600` is a directional/searchlight object with its own object lifecycle and active-list membership.

## 2. Class Layout / Key Offsets

| Offset / symbol | Type | Purpose | Evidence | Active in YR |
|---|---:|---|---|---|
| `BuildingTypeClass+0x154B` | byte bool | `HasSpotlight=` flag. Default false, parser preserves old value as default. | default write `MOV byte ptr [ESI+0x154b],BL` at `0x0045DFDA`; parser `CCINIClass__ReadBool(... "HasSpotlight", [EBP+0x154B])` at `0x0045FEE5..0x0045FEF4` | Conditional |
| `s_HasSpotlight_0081AEA0` | ascii string | INI key string. | string search result at `0x0081AEA0`; sole xref from `0x0045FEEC` | Conditional |
| `BuildingClass+0x600` | pointer | `BuildingLightClass*` spotlight pointer. | `BuildingClass__Unlimbo` writes `[ESI+0x600]` at `0x00441190` after allocation/constructor | Conditional |
| `BuildingClass+0x614` | pointer | `LightSourceClass*` ambience/radius light, separate from this path. | prior spotlight and light-source reports; `src` currently models only point lights | Conditional, non-scope |
| `BuildingLightClass+0xD8` | byte | parity/toggle byte set from `DAT_008B4194` index parity after constructor registration. | loop and `MOV byte ptr [ESI+0xD8],CL` at `0x00435B0F..0x00435B3F` | Conditional |
| `BuildingLightClass+0xDC` | int | mode; constructor calls `FindTarget(1)` after registration. | `PUSH 0x1; CALL 0x00436BE0` at `0x00435B06..0x00435B0A` | Conditional |
| `BuildingLightClass+0xE4` | pointer | owning building pointer. | constructor stores `param_2` at `param_1[0x39]`, byte offset `0xE4` | Conditional |
| `ObjectClass+0x98` | byte | `LogicClass` active-vector membership guard set by `FUN_0055BAA0`. | `FUN_0055BAA0` reads `[param+0x98]`, inserts, then writes `1` on success | Conditional |

## 3. Core Logic

### INI reader and default

Verified default:

- `BuildingTypeClass` initialization clears `+0x154B` with `BL` at `0x0045DFDA`. In the observed constructor/default block, `BL` is zero; nearby defaults set sibling bytes such as `+0x154A`, `+0x154C`, `+0x154D`, `+0x154F`.
- Assembly evidence: `0x0045DFD3` writes `+0x154A = 1`, then `0x0045DFDA` writes `+0x154B = BL`, then `0x0045DFE0` writes `+0x154C = BL`.

Verified reader:

- String search finds exactly one `HasSpotlight` string at `0x0081AEA0`.
- Xref evidence: `0x0081AEA0` is referenced only by `BuildingTypeClass_ReadINI_Water @ 0x0045FEEC`.
- Decompile evidence: `BuildingTypeClass_ReadINI_Water` calls `CCINIClass__ReadBool(iVar21, s_HasSpotlight_0081AEA0, *(byte *)(param_1+0x154B))`, then stores the result back to `param_1+0x154B`.
- Assembly evidence: `0x0045FEE5` loads old byte `[EBP+0x154B]`, `0x0045FEEC` pushes `"HasSpotlight"`, `0x0045FEF4` calls `CCINIClass__ReadBool`.

The key is therefore building-type scoped, not art scoped, not map-lighting scoped, and not a synonym for `LightIntensity`.

### Allocation gate in `BuildingClass__Unlimbo`

`BuildingClass__Unlimbo` reaches the spotlight allocation near the late post-unlimbo building side effects, after normal `TechnoClass__Unlimbo` success and after the ordinary `LightSourceClass` branch for ambience. The exact gate is:

1. Load type pointer from `BuildingClass+0x520`.
2. Read byte `[type+0x154B]`.
3. If zero, skip directly past the spotlight allocation and leave `BuildingClass+0x600` untouched by this branch.
4. If nonzero, allocate `0xE8` bytes.
5. If allocation succeeds, call `BuildingLightClass__Constructor` with the owning building.
6. Store returned pointer at `BuildingClass+0x600`; if allocation failed, store zero.

Assembly evidence:

- `0x00441163`: `MOV ECX,dword ptr [ESI+0x520]`
- `0x00441169`: `MOV AL,byte ptr [ECX+0x154B]`
- `0x0044116F..0x00441171`: `TEST AL,AL`; `JZ 0x00441196`
- `0x00441173`: `PUSH 0xE8`
- `0x00441178`: allocation call
- `0x00441184..0x00441187`: `PUSH ESI`; `CALL 0x00435820`
- `0x00441190`: `MOV dword ptr [ESI+0x600],EAX`

This proves the field identity and the allocation order. It also proves `+0x600` is not created for ordinary stock light-post ambience unless `HasSpotlight=yes` is present.

### Constructor registration order

`BuildingLightClass__Constructor @ 0x00435820` has two distinct registration steps:

1. It appends the object to `DAT_008B4194` / `DAT_008B41A0`, the class/global vector used for `BuildingLightClass` objects.
2. If the owning building pointer is non-null, it computes initial endpoint coordinates from owner coordinates and the current `RateTimer__Current` value.
3. It calls virtual `+0x1B4` to produce raw/reveal coordinates, then calls `ObjectClass__Reveal`.
4. If `ObjectClass__Reveal` succeeds, it explicitly calls `FUN_0055BAA0` on `ECX=0x87F778` with `unique_scan_flag=0`.
5. It then calls `BuildingLightClass__FindTarget(1)`.
6. It scans its own position in `DAT_008B4194`, folds the index with `0x80000001`, and writes `BuildingLightClass+0xD8` from the index parity.

Assembly evidence:

- Global vector append: `0x00435907..0x0043591C` increments `DAT_008B41A0` and writes `ESI` into `[DAT_008B4194 + old_count*4]`.
- Reveal call: `0x00435AF0` calls `ObjectClass__Reveal @ 0x005F4EC0`.
- Success gate: `0x00435AF5..0x00435AF7` tests `AL` and skips helper on failure.
- Direct active-vector helper call: `0x00435AF9` pushes `0`, `0x00435AFB` pushes `ESI`, `0x00435AFC` moves `ECX=0x87F778`, and `0x00435B01` calls `0x0055BAA0`.
- Mode initialization after registration: `0x00435B06..0x00435B0A` pushes `1` and calls `BuildingLightClass__FindTarget`.
- Parity byte after mode initialization: `0x00435B0F..0x00435B3F` scans `DAT_008B4194` and writes `[ESI+0xD8]`.

Important ordering detail: the direct `FUN_0055BAA0` call is after a successful `ObjectClass__Reveal`, but it is not the ordinary `ObjectClass::Reveal` type-gated registration branch itself. It is an explicit second call site in the constructor. `FUN_0055BAA0`'s `Object+0x98` guard prevents duplicate insertion if the object was already registered.

### Reveal wrapper registration

`FUN_00437050 @ 0x00437050` is a `BuildingLightClass` reveal wrapper reached from a vtable data xref at `0x007E3BA8`. It repeats the same shape as the constructor's reveal tail:

1. Call `ObjectClass__Reveal(param_2, param_3)`.
2. If reveal fails, return `0`.
3. If reveal succeeds, call `FUN_0055BAA0` with `ECX=0x87F778`, object pointer, and `unique_scan_flag=0`.
4. Return `1`.

Assembly evidence:

- `0x0043705F`: call `ObjectClass__Reveal`.
- `0x00437064..0x00437066`: test `AL`; jump to failure on zero.
- `0x00437068`: push `0`.
- `0x0043706A`: push `ESI`.
- `0x0043706B`: move `ECX=0x87F778`.
- `0x00437070`: call `FUN_0055BAA0`.
- `0x00437075`: return success byte `1`; `0x0043707B` returns failure byte `0`.

### `FUN_0055BAA0` insertion semantics

`FUN_0055BAA0 @ 0x0055BAA0` is the ordinary `LogicClass` active-vector insertion helper. For this slot, the relevant verified behavior is:

- Read `Object+0x98`.
- If already nonzero, return success without inserting.
- Otherwise call `DynamicVector__Insert(object, unique_scan_flag)`.
- If insert succeeds, write `Object+0x98 = 1` and return success.
- If insert fails, leave `Object+0x98` clear and return failure.

The constructor and reveal wrapper both pass `unique_scan_flag=0`, matching the last swarm's direct-caller mapping.

## 4. INI Keys

| Key | Scope | Binary storage | Default / stock data | Effect | Active in YR |
|---|---|---:|---|---|---|
| `HasSpotlight=` | Building type section | `BuildingTypeClass+0x154B` byte | default false (`0x0045DFDA`); no assignments in repo INI; no visible extracted retail-file matches | gates `BuildingClass+0x600` allocation on successful building unlimbo | Conditional |
| `SpotlightMovementRadius=` | `[General]` | `RulesClass+0x788` int | `rulesmd.ini:446 = 2000` | used by spotlight behavior after allocation | Conditional on a spotlight existing |
| `SpotlightLocationRadius=` | `[General]` | `RulesClass+0x78C` int | `rulesmd.ini:447 = 1000` | used by spotlight behavior after allocation | Conditional |
| `SpotlightSpeed=` | `[General]` | `RulesClass+0x790` double | `rulesmd.ini:445 = .015` | used by spotlight movement | Conditional |
| `SpotlightAcceleration=` | `[General]` | `RulesClass+0x798` double | `rulesmd.ini:448 = .0025` | used by spotlight movement | Conditional |
| `SpotlightAngle=` | `[General]` | `RulesClass+0x7A0` double | `rulesmd.ini:449 = .5` | used by spotlight sweep | Conditional |
| `SpotlightRadius=` | `[General]` | `RulesClass+0x7A8` int | not present in repo INI comments/data; binary still reads it from existing default | used by spotlight behavior | Conditional |

Reader assembly for `[General]` keys:

- `0x00671755`: pushes `SpotlightMovementRadius`, reads/stores `Rules+0x788`.
- `0x00671775`: pushes `SpotlightLocationRadius`, reads/stores `Rules+0x78C`.
- `0x0067179C`: pushes `SpotlightSpeed`, reads/stores double at `Rules+0x790`.
- `0x006717C3`: pushes `SpotlightAcceleration`, reads/stores double at `Rules+0x798`.
- `0x006717EA`: pushes `SpotlightAngle`, reads/stores double at `Rules+0x7A0`.
- `0x00671809`: pushes `SpotlightRadius`, reads/stores `Rules+0x7A8`.

The `[General]` keys do not activate the system by themselves; they only parameterize `BuildingLightClass` after a `HasSpotlight` building exists.

## 5. Integration Points

| Integration | Verified behavior | Evidence | Active in YR |
|---|---|---|---|
| Building type parse | `HasSpotlight` is read by `BuildingTypeClass_ReadINI_Water` and stored at `+0x154B`. | string xref `0x0045FEEC`; decompile and assembly `0x0045FEE5..0x0045FEF4` | Conditional |
| Building unlimbo | Checks `Type+0x154B`, allocates `0xE8`, calls constructor, stores `+0x600`. | `0x00441169..0x00441190` | Conditional |
| Constructor | Appends to `DAT_008B4194`, reveals, then directly appends to `LogicClass` active vector through `FUN_0055BAA0`. | `0x00435907..0x0043591C`; `0x00435AF0..0x00435B01` | Conditional |
| Reveal wrapper | Successful reveal is followed by direct `FUN_0055BAA0`, same helper args as constructor. | `FUN_00437050`; assembly `0x0043705F..0x00437075` | Conditional |
| Teardown | Destructor conceals, removes from `LogicClass` if conceal succeeded, removes from `DAT_008B4194`, then calls `ObjectClass` destructor. | `BuildingLightClass__Destructor @ 0x004370C0` decompile | Conditional |
| Save/load | Existing spotlight reports verify save/load/fixup for owner/target; not re-expanded in this slot. | `BUILDINGLIGHTCLASS_SPOTLIGHT_PATH_GHIDRA_REPORT.md` | Conditional |

## 6. Current Rust Implementation Status

Current Rust has point-light ambience, not `BuildingLightClass`:

- `src/rules/object_type.rs` parses `LightVisibility`, `LightIntensity`, and tint keys, but `rg` found no `HasSpotlight` parser or field.
- `src/map/lighting.rs` defines `PointLight` and `collect_building_lights`, with collection gated by nonzero `LightIntensity`.
- `src/app_init.rs` rebuilds the lighting grid from live structures by collecting point lights from `LightVisibility` / `LightIntensity`.
- No Rust symbol for `BuildingLightClass`, `HasSpotlight`, directional beam state, `BuildingClass+0x600` equivalent, or direct live-object registration path was found.

Rust should keep `HasSpotlight` separate from the point-light fields. `BuildingLightClass` is an object lifecycle/active-vector concern plus a render overlay/searchlight concern; it is not a map cell ambience contribution.

## 7. Coverage Ledger

| Area / function / branch | Status | Evidence | What remains |
|---|---|---|---|
| `HasSpotlight` string identity | verified | string `0x0081AEA0`; sole xref `0x0045FEEC` | none |
| `BuildingTypeClass+0x154B` default | verified | `0x0045DFDA` writes zero via `BL` | none |
| `HasSpotlight=` parser | verified | decompile `BuildingTypeClass_ReadINI_Water`; assembly `0x0045FEE5..0x0045FEF4` | none |
| `BuildingClass+0x600` allocation gate | verified | decompile `BuildingClass__Unlimbo`; assembly `0x00441169..0x00441190` | none |
| `BuildingLightClass` constructor registration | verified | decompile `0x00435820`; assembly `0x00435907..0x0043591C`, `0x00435AF0..0x00435B01` | none for registration order |
| `FUN_00437050` reveal wrapper | verified | vtable data xref `0x007E3BA8`; assembly `0x0043705F..0x00437075` | none |
| `FUN_0055BAA0` helper behavior | verified | decompile `0x0055BAA0`; prior helper report | none |
| `[General]` spotlight rule readers | verified | strings and xrefs `0x00671755..0x00671816` | none for key ownership |
| Standard repo INI activation | verified negative | `rg -i "HasSpotlight" ini/rules.ini ini/rulesmd.ini ini/art.ini ini/artmd.ini` returned no matches | none for repo INI |
| Configured retail visible-file activation | verified negative for visible files | `rg -i "HasSpotlight" "C:/Users/enok/Documents/Command and Conquer Red Alert II"` returned no matches | packed archive internals were not decoded in this slot |
| Rust parser/surface scan | verified | `rg` over `src`; `src/rules/object_type.rs`, `src/map/lighting.rs`, `src/app_init.rs` | implementation absent |
| Exact spotlight pixels/cell action | deferred | existing reports `BUILDINGLIGHTCLASS_BEAM_RASTERIZATION_AND_CELLACTION_0X23_GHIDRA_REPORT.md` | out-of-scope here; already covered elsewhere |

## 8. Open Questions -- Final State of the Investigation Log

- `[RESOLVED] OQ-BLHS-001 -- Which INI key controls the path? -> Building-type `HasSpotlight=`.` (evidence: `0x0081AEA0`, xref `0x0045FEEC`)
- `[RESOLVED] OQ-BLHS-002 -- What is the storage field? -> byte `BuildingTypeClass+0x154B`.` (evidence: default write `0x0045DFDA`; parser load/store `0x0045FEE5..0x0045FEF4`)
- `[RESOLVED] OQ-BLHS-003 -- What is the default? -> false/zero.` (evidence: `MOV byte ptr [ESI+0x154B],BL` at `0x0045DFDA` with `BL=0`)
- `[RESOLVED] OQ-BLHS-004 -- Does the parser use old value as default? -> Yes; it passes `[EBP+0x154B]` into `CCINIClass__ReadBool` before storing the result.` (evidence: `0x0045FEE5..0x0045FEF4`)
- `[RESOLVED] OQ-BLHS-005 -- Does any other binary reader use the string? -> No other xrefs were found for `0x0081AEA0`.` (evidence: Ghidra `get_xrefs_to 0x0081AEA0`)
- `[RESOLVED] OQ-BLHS-006 -- What object field receives the runtime pointer? -> `BuildingClass+0x600`.` (evidence: `0x00441190`)
- `[RESOLVED] OQ-BLHS-007 -- What is allocated? -> `0xE8` bytes for `BuildingLightClass`.` (evidence: `PUSH 0xE8` at `0x00441173`; constructor call `0x00441187`)
- `[RESOLVED] OQ-BLHS-008 -- Is allocation before or after normal building unlimbo success? -> After `TechnoClass__Unlimbo` succeeds and after several building-side effects; the branch is late in `BuildingClass__Unlimbo`.` (evidence: decompile `BuildingClass__Unlimbo`)
- `[RESOLVED] OQ-BLHS-009 -- Is `+0x600` the same as `LightSourceClass` ambience? -> No; `+0x614` is the separate point/radius light pointer, while `+0x600` is `BuildingLightClass*`.` (evidence: prior light-source docs plus `0x00441190`)
- `[RESOLVED] OQ-BLHS-010 -- Does constructor call ordinary `ObjectClass__Reveal`? -> Yes, after coordinate setup.` (evidence: `0x00435AF0`)
- `[RESOLVED] OQ-BLHS-011 -- Does constructor also call the active-vector helper directly? -> Yes, only when reveal returns success.` (evidence: `0x00435AF5..0x00435B01`)
- `[RESOLVED] OQ-BLHS-012 -- What helper args are used? -> `ECX=0x87F778`, object pointer `ESI`, `unique_scan_flag=0`.` (evidence: `0x00435AF9..0x00435B01`)
- `[RESOLVED] OQ-BLHS-013 -- What prevents duplicate live insertion? -> `FUN_0055BAA0` checks `Object+0x98` before insert and returns success if already set.` (evidence: decompile `0x0055BAA0`)
- `[RESOLVED] OQ-BLHS-014 -- Does `FUN_00437050` repeat this direct-registration shape? -> Yes; it calls `ObjectClass__Reveal`, then `FUN_0055BAA0` with flag `0` on success.` (evidence: decompile `FUN_00437050`; assembly `0x0043705F..0x00437075`)
- `[RESOLVED] OQ-BLHS-015 -- Does constructor initialize mode before or after active helper? -> After direct active-helper call, it calls `FindTarget(1)`.` (evidence: `0x00435B01` then `0x00435B06..0x00435B0A`)
- `[RESOLVED] OQ-BLHS-016 -- What standard data activates it? -> No `HasSpotlight=` assignments were found in repo INI or visible configured retail files.` (evidence: `rg -i "HasSpotlight"` scans)
- `[RESOLVED] OQ-BLHS-017 -- Are `[General]` spotlight keys real binary readers? -> Yes, six keys are read at `RulesClass+0x788..0x7A8`.` (evidence: string xrefs and assembly `0x00671755..0x00671816`)
- `[RESOLVED] OQ-BLHS-018 -- Does current Rust parse or model this? -> No; scans found point-light fields only and no `HasSpotlight`/`BuildingLightClass` surface.` (evidence: `rg` over `src`)
- `[DEFERRED] OQ-BLHS-019 -- Do packed retail MIX archives contain a mission/map override with `HasSpotlight=`?` (category: `out-of-scope`; reason: this slot checked repo INI and visible configured retail files, not archive extraction; next-step-if-pursued: scan extracted MIX map/mission INI payloads for `HasSpotlight=`)
- `[DEFERRED] OQ-BLHS-020 -- What exact pixels does the spotlight draw?` (category: `out-of-scope`; reason: registration gate/lifecycle slice only; next-step-if-pursued: use `BUILDINGLIGHTCLASS_BEAM_RASTERIZATION_AND_CELLACTION_0X23_GHIDRA_REPORT.md` and screenshot QA)

Adversarial corner cases answered:

- Missing key: parser preserves zero default and no spotlight is allocated. Evidence: `0x0045DFDA`, `0x0045FEE5..0x0045FEF4`, `0x00441169..0x00441171`.
- Allocation failure: `BuildingClass__Unlimbo` stores zero at `+0x600`. Evidence: `0x00441180..0x00441190`.
- Reveal failure in constructor/wrapper: the direct active-vector helper is skipped. Evidence: `0x00435AF5..0x00435AF7`, `0x00437064..0x00437066`.
- Duplicate registration: `Object+0x98` makes `FUN_0055BAA0` return success without another insert. Evidence: decompile `0x0055BAA0`.
- Stock rules with `Spotlight*` general keys but no `HasSpotlight`: parameters exist but no `BuildingLightClass` is created. Evidence: `0x00441169` gate and INI scans.

## 9. Visual/UI Composition Ledger

This report does not claim pixel composition. The scoped visual fact is only negative/ownership: `HasSpotlight` does not route through Rust's current point-light ambience model and should not be implemented as `LightVisibility`/`LightIntensity`.

| Order | Function / address | Condition / flag proof | Asset / frame | Rect / anchor | Palette / convert | Active for target? | Role |
|---|---|---|---|---|---|---|---|
| 1 | `BuildingClass__Unlimbo @ 0x00441169..0x00441190` | `Type+0x154B != 0` | none | none | none | conditional | creates spotlight object |
| 2 | `BuildingLightClass__Constructor @ 0x00435AF0..0x00435B01` | owner non-null and reveal success | none | object raw/reveal coords | none | conditional | active-list registration |
| deferred | beam draw helpers | out-of-scope here | runtime primitive/surface lines | screen-space beam | surface brighten path | conditional | see beam rasterization report |

Asset role matrix:

| Asset | Loaded | Drawn | Visible in target | Content/preview | Chrome/container | Overlay | Transition-only | Inactive | Evidence |
|---|---|---|---|---|---|---|---|---|
| `LightVisibility`/`LightIntensity` point-light fields | yes in Rust | yes as cell ambience | conditional for point-light buildings | no | no | no | no | inactive for `BuildingLightClass` | Rust scans; prior spotlight reports |
| `BuildingLightClass` spotlight primitive/beam | conditional | conditional | conditional when `HasSpotlight=yes` and draw gates pass | no | no | yes | no | inactive in stock repo INI | `0x00441169..0x00441190`; beam report |

## 10. Implementation Handoff

| Verified behavior | Evidence | Current Rust delta | Affected Rust surface | Required implementation effect | Acceptance scenario | Risk / do-not-do |
|---|---|---|---|---|---|---|
| `HasSpotlight=` is a building-type bool at `BuildingTypeClass+0x154B`, default false, parsed by `BuildingTypeClass_ReadINI_Water` with old value as default. | `0x0045DFDA`, `0x0045FEE5..0x0045FEF4`, string `0x0081AEA0` | missing | `src/rules/object_type.rs` or future building-type-specific rules surface | Carry a separate `has_spotlight` flag; default false; parse only from object/building type INI. | Rules fixture with no key yields false; fixture with `HasSpotlight=yes` yields true; unrelated `LightIntensity` does not affect it. | Do not infer this from building name, art keys, `LightIntensity`, or `LightVisibility`. |
| `BuildingClass__Unlimbo` allocates `0xE8` `BuildingLightClass` and stores `BuildingClass+0x600` only when `Type+0x154B != 0`. | `0x00441169..0x00441190` | missing | future building lifecycle/unlimbo surface; object runtime storage | Split ordinary building storage from optional spotlight object creation; create spotlight after successful unlimbo at the native point in order. | Custom building with `HasSpotlight=no` has no spotlight runtime; same building with `HasSpotlight=yes` gets one after successful unlimbo, not at type parse or map read time. | Do not create spotlights for all named "spotlight" buildings or all point-light buildings. |
| `BuildingLightClass` constructor appends to its class vector, reveals, then calls `FUN_0055BAA0` directly with flag `0` if reveal succeeds. | `0x00435907..0x0043591C`, `0x00435AF0..0x00435B01` | missing | future live-object/order model, `src/sim/world/mod.rs` active-order replacement | A spotlight object must become a live active object by the direct registration path and tail append order, guarded by object membership. | Two objects created in known order, one ordinary reveal and one HasSpotlight building, produce active order matching constructor/reveal timing; failed reveal does not append spotlight. | Do not rely on sorted entity IDs or map-storage order for spotlight AI order. |
| `FUN_00437050` reveal wrapper repeats reveal-success -> direct `FUN_0055BAA0(flag=0)` semantics. | `0x0043705F..0x00437075`, vtable xref `0x007E3BA8` | missing | future `BuildingLightClass` reveal/conceal API | Any later spotlight reveal path must call the same active-list helper after successful reveal. | Conceal/reveal a spotlight object; active membership is restored once, with `Object+0x98` preventing duplicates. | Do not assume only the constructor can register a spotlight into the live vector. |
| `FUN_0055BAA0` duplicate guard is `Object+0x98`, and insert failure leaves it clear. | decompile `0x0055BAA0`; helper report | future active-order mechanism mismatches | `src/sim/world/mod.rs` live-object membership model | Model active membership as a guardable object state, not merely presence in `EntityStore`. | Re-register an already live spotlight; active-order length does not grow. Force insert failure in a harness; membership remains false. | Do not append blindly on every reveal/constructor call. |
| Standard repo data does not activate the path, despite `[General]` spotlight parameters existing. | INI scans; `[General]` reader `0x00671755..0x00671816` | no stock-content behavior currently missing for unmodified INI, but mod/map parity missing | rules parser and optional runtime feature | Parse parameters separately, but only instantiate spotlight runtime when `HasSpotlight=yes`. | Standard rules load has zero HasSpotlight types; modded rules with one HasSpotlight type activates path. | Do not treat `SpotlightSpeed`/radius keys as activation gates. |
| Current Rust point lights are `LightIntensity`/`LightVisibility` ambience, not `BuildingLightClass`. | `src/map/lighting.rs`, `src/app_init.rs`, `src/rules/object_type.rs` scans | missing separate spotlight model | render overlay plus sim/object lifecycle | Keep point-light grid and spotlight overlay/searchlight lifecycle separate. | A building with only `LightIntensity` emits cell ambience but no beam; a building with only `HasSpotlight=yes` has spotlight lifecycle/beam without point-light ambience. | Do not implement `HasSpotlight` by creating a `PointLight`. |

### Stale Docs / Follow-up Docs

- `BUILDINGLIGHTCLASS_SPOTLIGHT_PATH_GHIDRA_REPORT.md` is directionally correct for this slot. Its "stock activation conditional because no shipped HasSpotlight assignments found" wording should be retained or strengthened to: "The parser/runtime path is live, but unmodified repo standard rules/art have no `HasSpotlight=` assignments; activation requires a map/mod/rules override."
- Any doc implying `BuildingClass+0x600` is the same as map cell lighting should use: "`BuildingClass+0x600` is `BuildingLightClass*` gated by `HasSpotlight`; `BuildingClass+0x614` / `LightSourceClass*` is the ordinary point/radius light path."
- Any implementation note should mention the direct `FUN_0055BAA0` call in both `BuildingLightClass__Constructor` and `FUN_00437050`; this is not solely ordinary `ObjectClass::Reveal` type-gated registration.

## Sources

- Ghidra read-only decompilations: `BuildingTypeClass_ReadINI_Water @ 0x0045FE50`, `BuildingClass__Unlimbo @ 0x00440580`, `BuildingLightClass__Constructor @ 0x00435820`, `FUN_00437050 @ 0x00437050`, `FUN_0055BAA0 @ 0x0055BAA0`, `BuildingLightClass__Destructor @ 0x004370C0`, `RulesClass__ReadGeneral @ 0x0066D530`.
- Ghidra read-only string/xref evidence: `HasSpotlight @ 0x0081AEA0`, xref `0x0045FEEC`; `Spotlight*` strings `0x0083B850..0x0083B8B0`, xrefs `0x00671755..0x00671809`; `BuildingLightClass__Constructor` xrefs from `0x00441187` and `0x006C0451`; `FUN_00437050` vtable data xref `0x007E3BA8`; `FUN_0055BAA0` xrefs include `0x00435B01` and `0x00437070`.
- Ghidra assembly contexts: `0x0045DFDA`, `0x0045FEE5..0x0045FEF4`, `0x00441169..0x00441190`, `0x00435907..0x0043591C`, `0x00435AF0..0x00435B01`, `0x00435B06..0x00435B3F`, `0x0043705F..0x00437075`, `0x00671755..0x00671816`.
- Prior reports referenced: `docs/research/BUILDINGLIGHTCLASS_SPOTLIGHT_PATH_GHIDRA_REPORT.md`, `docs/research/BUILDINGLIGHTCLASS_BEAM_RASTERIZATION_AND_CELLACTION_0X23_GHIDRA_REPORT.md`, `docs/research/LOGIC_OBJECT_REGISTRATION_HELPER_FUN_0055BAA0_GHIDRA_REPORT.md`, `docs/research/DIRECT_NON_REVEAL_FUN_0055BAA0_CALLERS_RESWARM_20260528.md`.
- INI scans: `ini/rules.ini`, `ini/rulesmd.ini`, `ini/art.ini`, `ini/artmd.ini`; visible configured retail files under `C:/Users/enok/Documents/Command and Conquer Red Alert II/`.
- Rust scans: `src/rules/object_type.rs`, `src/map/lighting.rs`, `src/app_init.rs`, plus `rg` over `src` for `HasSpotlight`, `BuildingLight`, and spotlight terms.
